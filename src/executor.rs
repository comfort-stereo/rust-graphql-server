use anyhow::Result;
use chrono::Utc;
use std::time::Duration;
use tide::log;

use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use rand::Rng;
use redis::{aio::ConnectionManager, AsyncCommands};
use sqlx::{query, query_as, PgPool};
use uuid::Uuid;

use crate::{
    auth::{SessionToken, SessionTokenData},
    config::Config,
    models::User,
    state::State,
};

/// The business logic handler for a request.
pub struct Executor {
    state: State,
}

impl Executor {
    /// Create a new executor with access to the global server state.
    pub fn new(state: State) -> Self {
        Self { state }
    }

    /// Access the server configuration settings.
    fn config(&self) -> &Config {
        &self.state.config
    }

    /// Access the Postgres database connection pool.
    fn db(&self) -> &PgPool {
        &self.state.db
    }

    /// Access the Redis database connection manager.
    fn redis(&self) -> ConnectionManager {
        self.state.redis.clone()
    }

    /// Attempt to create a new user with the provided username, email and password. Once the user
    /// is created, an email verification code will be sent to the user's email address. That same
    /// verification code is stored temporarily in the Redis database until the code expires. To
    /// verify a user's email address, we just make sure the verification code the user sends in
    /// later matches the code we have stored in Redis.
    pub async fn create_user(&self, username: &str, email: &str, password: &str) -> Result<User> {
        let Config {
            password_hash_cost, ..
        } = self.config();

        let id = Uuid::new_v4();
        let password_hash = bcrypt::hash(password, *password_hash_cost)?;

        // Create the user.
        let user = query_as!(
            User,
            "
            INSERT INTO users (id, username, email, password_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            ",
            id,
            username,
            email,
            password_hash,
        )
        .fetch_one(self.db())
        .await?;

        // Create a new verification code.
        let verification_code = self.generate_verification_code();

        // Put the verification code in the Redis database.
        log::info!("Registering email verification code: {}", verification_code);
        self.register_email_verification_code(id, email, &verification_code)
            .await?;

        // Send the same verification code to the user's email address.
        log::info!("Sending email verification code: {}", verification_code);
        if self
            .send_email_verification_code(username, email, &verification_code)
            .await
            .is_err()
        {
            log::error!(
                "Failed to send email verification code: {}",
                verification_code
            );
        }

        Ok(user)
    }

    /// Create a new user-friendly verification code. As of now, these are just a 6 character long
    /// strings of upper-case letters.
    fn generate_verification_code(&self) -> String {
        let mut rng = rand::thread_rng();
        (0..6).map(|_| rng.gen_range('A'..'Z')).collect()
    }

    /// Create the key a verification code can be stored under in the Redis database.
    fn create_email_verification_key(&self, user_id: Uuid, email: &str) -> String {
        format!("verify/{}/{}", user_id, email)
    }

    /// Put a new email verification code into the Regis database. The time it takes for the
    /// verification code to expire is specified by the EMAIL_VERIFICATION_CODE_EXPIRATION_SECONDS
    /// environment variable.
    async fn register_email_verification_code(
        &self,
        user_id: Uuid,
        email: &str,
        verification_code: &str,
    ) -> Result<()> {
        let Config {
            email_verification_code_expiration_seconds,
            ..
        } = self.config();
        let verification_key = self.create_email_verification_key(user_id, email);

        self.redis()
            .set_ex::<String, String, ()>(
                verification_key,
                verification_code.into(),
                *email_verification_code_expiration_seconds as usize,
            )
            .await?;

        Ok(())
    }

    /// Send an email verification code to a user via email. Email settings are defined by the
    /// server configuration.
    async fn send_email_verification_code(
        &self,
        username: &str,
        email: &str,
        verification_code: &str,
    ) -> Result<()> {
        let Config {
            email_smtp,
            email_smtp_port,
            email_smtp_use_starttls,
            email_verification_email_address,
            email_verification_email_password,
            ..
        } = self.config();

        let message = Message::builder()
            .from(format!("rust-graphql-server <{}>", email_verification_email_address).parse()?)
            .to(format!("{} <{}>", username, email).parse()?)
            .subject("Verify your account")
            .body(format!("Your verification code is: {}", verification_code))?;

        let relay = if *email_smtp_use_starttls {
            SmtpTransport::starttls_relay(email_smtp)?
        } else {
            SmtpTransport::relay(email_smtp)?
        };

        let mailer = relay
            .port(*email_smtp_port)
            .credentials(Credentials::new(
                email_verification_email_address.clone(),
                email_verification_email_password.clone(),
            ))
            .timeout(Some(Duration::from_secs(10)))
            .build();

        mailer.send(&message)?;
        Ok(())
    }

    /// Attempt to verify a user's email address using the provided verification code. This function
    /// will return true if the verification is successful and false otherwise. The verification
    /// will fail if the user does not exist or the verification code is invalid.
    pub async fn verify_user_email_address(
        &self,
        user_id: Uuid,
        verification_code: &str,
    ) -> Result<bool> {
        // Try to find the user. Return false if they don't exist.
        let user = match self.find_user(user_id).await? {
            Some(user) => user,
            None => return Ok(false),
        };

        let verification_key = self.create_email_verification_key(user.id, &user.email);

        // Try to retrieve the stored verification code.
        let stored_verification_code = self
            .redis()
            .get::<String, Option<String>>(verification_key.clone())
            .await?;

        // Verify the stored code matches the one passed in.
        if stored_verification_code == Some(verification_code.into()) {
            // Delete the verification code from Redis. We don't need it any more.
            self.redis().del::<String, ()>(verification_key).await?;

            // Mark the user as having a verified email.
            let email_verified_at = Some(Utc::now());
            query!(
                "UPDATE users SET email_verified_at = $1 WHERE id = $2",
                email_verified_at,
                user_id,
            )
            .execute(self.db())
            .await?;

            // Return true. We verified the email successfully.
            Ok(true)
        } else {
            // Return false. The email verification failed. We didn't have a matching verification
            // code stored.
            Ok(false)
        }
    }

    // Attempt to log in using the provided credentials. If successful return a session token to be
    // sent along with future requests. Otherwise return nothing.
    pub async fn login(&self, username: &str, password: &str) -> Result<Option<SessionToken>> {
        if let Some(User {
            id, password_hash, ..
        }) = &self.find_user_by_username(username).await?
        {
            if bcrypt::verify(password, password_hash)? {
                Ok(Some(self.create_session(*id).await?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Attempt to refresh a session token. The current session token will be used to create a new
    /// session token with an extended lifespan. The current session token will be invalidated and
    /// the new, refreshed token will be returned. No token will be returned if the provided session
    /// token is invalid.
    pub async fn refresh(&self, unverified_session_token: &str) -> Result<Option<SessionToken>> {
        let Config {
            session_token_secret,
            session_token_expiration_seconds,
            ..
        } = self.config();

        if let Some(SessionTokenData {
            session_id,
            user_id,
            ..
        }) = SessionToken::decode(unverified_session_token, session_token_secret)
        {
            if let Some(current_session_token) = self.find_session(session_id).await? {
                if current_session_token.to_string() != unverified_session_token {
                    return Ok(None);
                }

                let refreshed_session_token = SessionToken::encode(
                    SessionTokenData {
                        session_id,
                        session_token_id: Uuid::new_v4(),
                        user_id,
                    },
                    session_token_secret,
                );

                self.redis()
                    .set_ex::<String, String, ()>(
                        session_id.to_string(),
                        refreshed_session_token.to_string(),
                        *session_token_expiration_seconds as usize,
                    )
                    .await?;

                Ok(Some(refreshed_session_token))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Use the provided session token to terminate a session. This function will return true if the
    /// session is terminated successfully and false otherwise. The logout will fail and return
    /// false if the session token is invalid.
    pub async fn logout(&self, unverified_session_token: &str) -> Result<bool> {
        let Config {
            session_token_secret,
            ..
        } = self.config();

        if let Some(SessionTokenData { session_id, .. }) =
            SessionToken::decode(unverified_session_token, session_token_secret)
        {
            self.delete_session(session_id).await
        } else {
            Ok(false)
        }
    }

    /// Find a session by ID and return its associated session token. This will return none if the
    /// session does not exist.
    async fn find_session(&self, session_id: Uuid) -> Result<Option<SessionToken>> {
        let Config {
            session_token_secret,
            ..
        } = self.config();

        Ok(self
            .redis()
            .get::<String, Option<String>>(session_id.to_string())
            .await?
            .map(|session_token| SessionToken::verify(&session_token, session_token_secret))
            .flatten())
    }

    /// Create a session for the specified user. The returned token includes the session ID, the
    /// user's ID and a unique session token ID.
    async fn create_session(&self, user_id: Uuid) -> Result<SessionToken> {
        let Config {
            session_token_secret,
            ..
        } = self.config();

        let session_id = Uuid::new_v4();
        let session_token_id = Uuid::new_v4();
        let session_token_data = SessionTokenData {
            session_id,
            session_token_id,
            user_id,
        };

        let Config {
            session_token_expiration_seconds,
            ..
        } = self.config();

        let session_token = SessionToken::encode(session_token_data, session_token_secret);

        self.redis()
            .set_ex::<String, String, ()>(
                session_id.to_string(),
                session_token.to_string(),
                *session_token_expiration_seconds as usize,
            )
            .await?;

        Ok(session_token)
    }

    /// Terminate a session by ID. This will return true if the session was found and deleted. False
    /// will be returned otherwise.
    async fn delete_session(&self, session_id: Uuid) -> Result<bool> {
        let count = self
            .redis()
            .del::<String, u32>(session_id.to_string())
            .await?;

        Ok(count != 0)
    }

    /// Find a user by ID. This will return none if the user is not found.
    pub async fn find_user(&self, id: Uuid) -> Result<Option<User>> {
        Ok(query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_optional(self.db())
            .await?)
    }

    /// Find a user by their username. This will return none if no user has the specified username.
    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>> {
        Ok(
            query_as!(User, "SELECT * FROM users WHERE username = $1", username)
                .fetch_optional(self.db())
                .await?,
        )
    }

    /// Find users. As of now this just returns a list of all users. It should really be paginated
    /// and have parameters.
    pub async fn find_users(&self) -> Result<Vec<User>> {
        Ok(query_as!(User, "SELECT * FROM users ORDER BY created_at")
            .fetch_all(self.db())
            .await?)
    }
}
