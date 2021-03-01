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

pub struct Executor {
    state: State,
}

impl Executor {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    fn config(&self) -> &Config {
        &self.state.config
    }

    fn db(&self) -> &PgPool {
        &self.state.db
    }

    fn redis(&self) -> ConnectionManager {
        self.state.redis.clone()
    }

    pub async fn create_user(&self, username: &str, email: &str, password: &str) -> Result<User> {
        let Config {
            password_hash_cost, ..
        } = self.config();

        let id = Uuid::new_v4();
        let password_hash = bcrypt::hash(password, *password_hash_cost)?;
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

        let verification_code = self.generate_verification_code();

        log::info!("Registering email verification code: {}", verification_code);
        self.register_email_verification_code(id, email, &verification_code)
            .await?;

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

    fn generate_verification_code(&self) -> String {
        let mut rng = rand::thread_rng();
        (0..6).map(|_| rng.gen_range('A'..'Z')).collect()
    }

    fn create_email_verification_key(&self, user_id: Uuid, email: &str) -> String {
        format!("verify/{}/{}", user_id, email)
    }

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
            .from(format!("Amble <{}>", email_verification_email_address).parse()?)
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

    pub async fn verify_user_email_address(
        &self,
        user_id: Uuid,
        verification_code: &str,
    ) -> Result<bool> {
        let user = match self.find_user(user_id).await? {
            Some(user) => user,
            None => return Ok(false),
        };

        let verification_key = self.create_email_verification_key(user.id, &user.email);

        let stored_verification_code = self
            .redis()
            .get::<String, Option<String>>(verification_key.clone())
            .await?;

        if stored_verification_code == Some(verification_code.into()) {
            self.redis().del::<String, ()>(verification_key).await?;

            let email_verified_at = Some(Utc::now());
            let result = query!(
                "UPDATE users SET email_verified_at = $1 WHERE id = $2",
                email_verified_at,
                user_id,
            )
            .execute(self.db())
            .await?;

            Ok(result.rows_affected() != 0)
        } else {
            Ok(false)
        }
    }

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

    async fn delete_session(&self, session_id: Uuid) -> Result<bool> {
        let count = self
            .redis()
            .del::<String, u32>(session_id.to_string())
            .await?;

        Ok(count != 0)
    }

    pub async fn find_user(&self, id: Uuid) -> Result<Option<User>> {
        Ok(query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_optional(self.db())
            .await?)
    }

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>> {
        Ok(
            query_as!(User, "SELECT * FROM users WHERE username = $1", username)
                .fetch_optional(self.db())
                .await?,
        )
    }
}
