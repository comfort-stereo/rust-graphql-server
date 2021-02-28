use chrono::{DateTime, Utc};
use juniper::graphql_object;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub username: String,
    pub email: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub password_hash: String,
}

#[graphql_object]
impl User {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn email_verified_at(&self) -> &Option<DateTime<Utc>> {
        &self.email_verified_at
    }
}
