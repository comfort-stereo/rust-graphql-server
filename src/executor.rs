use std::collections::HashMap;

use async_trait::async_trait;
use dataloader::{non_cached::Loader, BatchFn};
use redis::{aio::ConnectionManager, AsyncCommands};
use sqlx::{query_as, PgPool};
use uuid::Uuid;

use crate::{
    auth::{SessionToken, SessionTokenData},
    config::Config,
    models::User,
    state::State,
};

pub struct Executor {
    state: State,
    users: UserLoader,
}

impl Executor {
    pub fn new(state: State) -> Self {
        let users = UserLoader::new(UserBatcher::new(state.db.clone())).with_yield_count(100);
        Self { state, users }
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

    pub async fn create_user(&self, username: &str, password: &str) -> Option<Uuid> {
        let Config {
            password_hash_cost, ..
        } = self.config();

        let id = Uuid::new_v4();
        let password_hash = bcrypt::hash(password, *password_hash_cost).unwrap();
        query_as!(
            User,
            "INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)",
            id,
            username,
            password_hash,
        )
        .execute(self.db())
        .await
        .map(|_| id)
        .ok()
    }

    pub async fn login(&self, username: &str, password: &str) -> Option<SessionToken> {
        if let Some(User {
            id, password_hash, ..
        }) = &self.find_user_by_username(username).await
        {
            if bcrypt::verify(password, password_hash).unwrap() {
                let session_token = self.create_session(*id).await;
                Some(session_token)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub async fn refresh(&self, unverified_session_token: &str) -> Option<SessionToken> {
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
            if let Some(current_session_token) = self.find_session(session_id).await {
                if current_session_token.to_string() != unverified_session_token {
                    return None;
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
                    .set_ex::<String, String, String>(
                        session_id.to_string(),
                        refreshed_session_token.to_string(),
                        *session_token_expiration_seconds as usize,
                    )
                    .await
                    .expect("to refresh session token");

                Some(refreshed_session_token)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub async fn logout(&self, unverified_session_token: &str) -> bool {
        let Config {
            session_token_secret,
            ..
        } = self.config();

        if let Some(SessionTokenData { session_id, .. }) =
            SessionToken::decode(unverified_session_token, session_token_secret)
        {
            self.delete_session(session_id).await
        } else {
            false
        }
    }

    async fn find_session(&self, session_id: Uuid) -> Option<SessionToken> {
        let Config {
            session_token_secret,
            ..
        } = self.config();

        self.redis()
            .get::<String, String>(session_id.to_string())
            .await
            .map(|session_token| SessionToken::verify(&session_token, session_token_secret))
            .ok()
            .flatten()
    }

    async fn create_session(&self, user_id: Uuid) -> SessionToken {
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
            .set_ex::<String, String, String>(
                session_id.to_string(),
                session_token.to_string(),
                *session_token_expiration_seconds as usize,
            )
            .await
            .unwrap();

        session_token
    }

    async fn delete_session(&self, session_id: Uuid) -> bool {
        self.redis()
            .del::<String, String>(session_id.to_string())
            .await
            .is_ok()
    }

    pub async fn find_user(&self, id: Uuid) -> Option<User> {
        self.users.load(id).await
    }

    pub async fn find_user_by_username(&self, username: &str) -> Option<User> {
        query_as!(User, "SELECT * FROM users WHERE username = $1", username)
            .fetch_optional(self.db())
            .await
            .unwrap()
    }
}

pub struct UserBatcher {
    db: PgPool,
}

impl UserBatcher {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BatchFn<Uuid, Option<User>> for UserBatcher {
    async fn load(&mut self, ids: &[Uuid]) -> HashMap<Uuid, Option<User>> {
        ids.iter()
            .map(|id| (*id, None))
            .chain(
                query_as!(User, "SELECT * FROM users WHERE id = ANY($1)", &ids)
                    .fetch_all(&self.db)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|user| (user.id, Some(user))),
            )
            .collect()
    }
}

pub type UserLoader = Loader<Uuid, Option<User>, UserBatcher>;
