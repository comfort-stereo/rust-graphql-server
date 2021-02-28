use juniper::{
    graphql_object, graphql_value, EmptySubscription, FieldError, FieldResult, RootNode,
};
use lazy_static::lazy_static;
use uuid::Uuid;

use crate::context::Context;
use crate::models::User;

pub struct Query;

const MIN_PASSWORD_LENGTH: usize = 6;
const MAX_PASSWORD_LENGTH: usize = 255;

#[graphql_object(context = Context)]
impl Query {
    async fn user(&self, context: &Context, id: Uuid) -> Option<User> {
        context.executor().find_user(id).await
    }

    async fn user_by_username(&self, context: &Context, username: String) -> Option<User> {
        context.executor().find_user_by_username(&username).await
    }
}

pub struct Mutation;

#[graphql_object(context = Context)]
impl Mutation {
    async fn login(
        &self,
        context: &Context,
        username: String,
        password: String,
    ) -> FieldResult<AuthResult> {
        if let Some(session_token) = context.executor().login(&username, &password).await {
            return Ok(AuthResult {
                session_token: session_token.to_string(),
            });
        }

        Err(FieldError::new(
            "Invalid username or password.",
            graphql_value!({ "code": "invalid-login" }),
        ))
    }

    async fn refresh(&self, context: &Context, session_token: String) -> FieldResult<AuthResult> {
        if let Some(session_token) = context.executor().refresh(&session_token).await {
            return Ok(AuthResult {
                session_token: session_token.to_string(),
            });
        }

        Err(FieldError::new(
            "Invalid session token.",
            graphql_value!({ "code": "invalid-session-token" }),
        ))
    }

    async fn logout(&self, context: &Context, session_token: String) -> bool {
        context.executor().logout(&session_token).await
    }

    async fn create_user(
        &self,
        context: &Context,
        username: String,
        email: String,
        password: String,
    ) -> FieldResult<User> {
        if context
            .executor()
            .find_user_by_username(&username)
            .await
            .is_some()
        {
            return Err(FieldError::new(
                "Username is already in use.",
                graphql_value!({ "code": "username-taken" }),
            ));
        }

        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(FieldError::new(
                "Password must be at least 6 characters.",
                graphql_value!({ "code": "password-too-short" }),
            ));
        }

        if password.len() > MAX_PASSWORD_LENGTH {
            return Err(FieldError::new(
                "Password cannot exceed 255 characters.",
                graphql_value!({ "code": "password-too-long" }),
            ));
        }

        if let Some(user) = context
            .executor()
            .create_user(&username, &email, &password)
            .await
        {
            Ok(user)
        } else {
            Err(FieldError::new(
                "Failed to create user.",
                graphql_value!({ "code": "unknown" }),
            ))
        }
    }

    async fn verify_user_email_address(
        &self,
        context: &Context,
        user_id: Uuid,
        verification_code: String,
    ) -> bool {
        context
            .executor()
            .verify_user_email_address(user_id, &verification_code)
            .await
    }
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

lazy_static! {
    pub static ref SCHEMA: Schema =
        Schema::new(Query, Mutation, EmptySubscription::<Context>::new());
}

#[derive(Debug, Clone)]
pub struct AuthResult {
    session_token: String,
}

#[graphql_object]
impl AuthResult {
    pub fn session_token(&self) -> &str {
        &self.session_token
    }
}
