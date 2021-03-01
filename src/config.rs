use std::str::FromStr;

use tide::log;

use crate::auth::{SessionToken, SessionTokenSecret};

// Names of server-relevant environment variables.
const PORT_VARIABLE: &str = "PORT";
const DATABASE_URL_VARIABLE: &str = "DATABASE_URL";
const DATABASE_MAX_CONNECTION_COUNT_VARIABLE: &str = "DATABASE_MAX_CONNECTION_COUNT";
const REDIS_URL_VARIABLE: &str = "REDIS_URL";
const SESSION_TOKEN_SECRET_VARIABLE: &str = "SESSION_TOKEN_SECRET";
const SESSION_TOKEN_EXPIRATION_SECONDS_VARIABLE: &str = "SESSION_TOKEN_EXPIRATION_SECONDS";
const PASSWORD_HASH_COST_VARIABLE: &str = "PASSWORD_HASH_COST";
const EMAIL_SMTP_VARIABLE: &str = "EMAIL_SMTP";
const EMAIL_SMTP_PORT_VARIABLE: &str = "EMAIL_SMTP_PORT";
const EMAIL_SMTP_USE_STARTTLS_VARIABLE: &str = "EMAIL_SMTP_USE_STARTTLS";
const EMAIL_VERIFICATION_EMAIL_ADDRESS_VARIABLE: &str = "EMAIL_VERIFICATION_EMAIL_ADDRESS";
const EMAIL_VERIFICATION_EMAIL_PASSWORD_VARIABLE: &str = "EMAIL_VERIFICATION_EMAIL_PASSWORD";
const EMAIL_VERIFICATION_CODE_EXPIRATION_SECONDS_VARIABLE: &str =
    "EMAIL_VERIFICATION_CODE_EXPIRATION_SECONDS";
const IS_DOCKER_VARIABLE: &str = "IS_DOCKER";

/// Configuration for the server. Each field is derived from an environment variable found on the
/// host or in local ".env" and ".env.override" files.
#[derive(Debug, Clone)]
pub struct Config {
    /// The port the server will run on.
    pub port: u16,
    /// A connection string for a Postgres database.
    pub database_url: String,
    /// The max number of pooled connections the server will maintain with the database.
    pub database_max_connection_count: u32,
    /// A connection string for a Redis database.
    pub redis_url: String,
    /// A secret used to generate/validate session tokens.
    pub session_token_secret: SessionTokenSecret,
    /// The number of seconds it takes for a session token to expire.
    pub session_token_expiration_seconds: u32,
    /// An integer specifying the cost of password hashing algorithm. See the "bcrypt" crate for
    /// more info.
    pub password_hash_cost: u32,
    /// The SMTP email server to use for sending emails.
    pub email_smtp: String,
    /// The port of the SMTP email server to connect to.
    pub email_smtp_port: u16,
    /// Specifies if the server should use the "STARTTLS" protocol for SMTP.
    pub email_smtp_use_starttls: bool,
    /// The email account used to send email verification codes.
    pub email_verification_email_address: String,
    /// The password for the email account used to send email verification codes.
    pub email_verification_email_password: String,
    /// The number of seconds it takes for an email verification code to expire.
    pub email_verification_code_expiration_seconds: u32,
    /// Set to true if the server is running in a Docker container.
    pub is_docker: bool,
}

impl Config {
    /// Load server configuration from environment variables and ".env" and ".env.override" files.
    pub async fn load() -> Self {
        if dotenv::from_filename(".env.override").is_ok() {
            log::info!("Loaded environment variables from '.env.override' file.");
        }
        if dotenv::from_filename(".env").is_ok() {
            log::info!("Loaded environment variables from '.env' file.");
        }

        let is_docker = var(IS_DOCKER_VARIABLE);
        let database_url = if is_docker {
            var::<String>(DATABASE_URL_VARIABLE).replace("localhost", "host.docker.internal")
        } else {
            var(DATABASE_URL_VARIABLE)
        };
        let redis_url = if is_docker {
            var::<String>(REDIS_URL_VARIABLE).replace("localhost", "host.docker.internal")
        } else {
            var(REDIS_URL_VARIABLE)
        };

        Config {
            port: var(PORT_VARIABLE),
            database_url,
            database_max_connection_count: var(DATABASE_MAX_CONNECTION_COUNT_VARIABLE),
            redis_url,
            session_token_secret: SessionToken::secret(&var::<String>(
                SESSION_TOKEN_SECRET_VARIABLE,
            )),
            session_token_expiration_seconds: var(SESSION_TOKEN_EXPIRATION_SECONDS_VARIABLE),
            password_hash_cost: var(PASSWORD_HASH_COST_VARIABLE),
            email_smtp: var(EMAIL_SMTP_VARIABLE),
            email_smtp_port: var(EMAIL_SMTP_PORT_VARIABLE),
            email_smtp_use_starttls: var(EMAIL_SMTP_USE_STARTTLS_VARIABLE),
            email_verification_email_address: var(EMAIL_VERIFICATION_EMAIL_ADDRESS_VARIABLE),
            email_verification_email_password: var(EMAIL_VERIFICATION_EMAIL_PASSWORD_VARIABLE),
            email_verification_code_expiration_seconds: var(
                EMAIL_VERIFICATION_CODE_EXPIRATION_SECONDS_VARIABLE,
            ),
            is_docker,
        }
    }
}

/// Get an environment variable and try to parse it as a specified data type. This function will
// panic if the variable cannot be found or cannot be parsed.
fn var<T: FromStr>(name: &str) -> T {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("Missing environment variable: {}", name))
        .parse()
        .unwrap_or_else(|_| panic!("Failed to parse environment variable: {}", name))
}
