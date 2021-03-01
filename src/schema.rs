use anyhow::Result;
use juniper::{
    graphql_object, graphql_value, EmptySubscription, FieldError, FieldResult, RootNode,
};
use lazy_static::lazy_static;
use tide::log;
use uuid::Uuid;

use crate::context::Context;
use crate::models::User;

/// Queries for the GraphQL schema.
pub struct Query;

/// Minimum password length for a user's password.
const MIN_PASSWORD_LENGTH: usize = 6;
/// Maximum password length for a user's password.
const MAX_PASSWORD_LENGTH: usize = 255;

/// Convert a generic "anyhow" result into a GraphQL field result.
fn convert_result<T>(result: Result<T>) -> FieldResult<T> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            log::error!("{}", error);
            Err(FieldError::new(
                "An unknown error occurred.",
                graphql_value!({ "code": "unknown-error" }),
            ))
        }
    }
}

#[graphql_object(context = Context, description="All available GraphQL queries.")]
impl Query {
    #[graphql(
        description = "Find a user by their ID.",
        arguments(id(description = "The user's ID."))
    )]
    async fn user(&self, context: &Context, id: Uuid) -> FieldResult<Option<User>> {
        convert_result(context.executor().find_user(id).await)
    }

    #[graphql(
        description = "Find a user by their username.",
        arguments(username(description = "The user's username."))
    )]
    async fn user_by_username(
        &self,
        context: &Context,
        username: String,
    ) -> FieldResult<Option<User>> {
        convert_result(context.executor().find_user_by_username(&username).await)
    }
}

/// Mutations for the GraphQL schema.
pub struct Mutation;

#[graphql_object(context = Context, description="All available GraphQL mutations.")]
impl Mutation {
    #[graphql(
        description = "Log in using a specified username and password.",
        arguments(
            username(description = "The username of the user to log in as."),
            password(description = "The user's password"),
        )
    )]
    async fn login(
        &self,
        context: &Context,
        username: String,
        password: String,
    ) -> FieldResult<AuthResult> {
        if let Some(session_token) =
            convert_result(context.executor().login(&username, &password).await)?
        {
            return Ok(AuthResult {
                session_token: session_token.to_string(),
            });
        }

        Err(FieldError::new(
            "Invalid username or password.",
            graphql_value!({ "code": "invalid-login" }),
        ))
    }

    #[graphql(
        description = "Attempt to refresh an active session using a session token. If successful,
        the lifespan of the session will be extended, the current session token will be invalidated,
        and a new session token will be returned for future authentication.",
        arguments(session_token(description = "The session token to refresh."))
    )]
    async fn refresh(&self, context: &Context, session_token: String) -> FieldResult<AuthResult> {
        if let Some(session_token) =
            convert_result(context.executor().refresh(&session_token).await)?
        {
            return Ok(AuthResult {
                session_token: session_token.to_string(),
            });
        }

        Err(FieldError::new(
            "Invalid session token.",
            graphql_value!({ "code": "invalid-session-token" }),
        ))
    }

    #[graphql(
        description = "Terminate the session associated with a specified session token. The token
        will be invalidated so it cannot be used for future authentication. This will return true
        if the specified session token was valid and the log out operation was successful.",
        arguments(session_token(description = "The session token to invalidate."))
    )]
    async fn logout(&self, context: &Context, session_token: String) -> FieldResult<bool> {
        convert_result(context.executor().logout(&session_token).await)
    }

    #[graphql(
        description = "Attempt to create a new user with the provided username, email and password.
        Once the user is created, an email verification code will be sent to the user's email
        address.",
        arguments(username(description = "The user's username.")),
        arguments(email(description = "The user's email.")),
        arguments(email(description = "The password the user will use to log in."))
    )]
    async fn create_user(
        &self,
        context: &Context,
        username: String,
        email: String,
        password: String,
    ) -> FieldResult<User> {
        if username.is_empty() {
            return Err(FieldError::new(
                "Username cannot be empty.",
                graphql_value!({ "code": "username-empty" }),
            ));
        }

        if convert_result(context.executor().find_user_by_username(&username).await)?.is_some() {
            return Err(FieldError::new(
                "Username is already in use.",
                graphql_value!({ "code": "username-taken" }),
            ));
        }

        if email.is_empty() {
            return Err(FieldError::new(
                "Email cannot be empty.",
                graphql_value!({ "code": "email-empty" }),
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

        convert_result(
            context
                .executor()
                .create_user(&username, &email, &password)
                .await,
        )
    }

    #[graphql(
        description = "Verify the current email address of a user. This will return true if the
        verification code was valid and the email address was verified successfully.",
        arguments(
            user_id(description = "The ID of the user to verify."),
            verification_code(description = "The verification code that was emailed to the user."),
        )
    )]
    async fn verify_user_email_address(
        &self,
        context: &Context,
        user_id: Uuid,
        verification_code: String,
    ) -> FieldResult<bool> {
        convert_result(
            context
                .executor()
                .verify_user_email_address(user_id, &verification_code)
                .await,
        )
    }
}

/// Type of the executable GraphQL schema.
pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

lazy_static! {
    /// Static immutable reference to the executable GraphQL schema.
    pub static ref SCHEMA: Schema =
        Schema::new(Query, Mutation, EmptySubscription::<Context>::new());
}

#[derive(Debug, Clone)]
pub struct AuthResult {
    session_token: String,
}

#[graphql_object(description = "The result of a successful authentication action.")]
impl AuthResult {
    #[graphql(
        description = "The session token to be used for future requests. This should be sent as a
        bearer token in the 'authorization' header."
    )]
    pub fn session_token(&self) -> &str {
        &self.session_token
    }
}
