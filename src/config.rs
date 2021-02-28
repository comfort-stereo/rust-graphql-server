use std::str::FromStr;

use tide::log;

use crate::auth::{SessionToken, SessionTokenSecret};

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

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub database_max_connection_count: u32,
    pub redis_url: String,
    pub session_token_secret: SessionTokenSecret,
    pub session_token_expiration_seconds: u32,
    pub password_hash_cost: u32,
    pub email_smtp: String,
    pub email_smtp_port: u16,
    pub email_smtp_use_starttls: bool,
    pub email_verification_email_address: String,
    pub email_verification_email_password: String,
    pub email_verification_code_expiration_seconds: u32,
}

impl Config {
    pub async fn load() -> Self {
        if dotenv::from_filename(".env.override").is_ok() {
            log::info!("Loaded environment variables from '.env.override' file.");
        }
        if dotenv::from_filename(".env").is_ok() {
            log::info!("Loaded environment variables from '.env' file.");
        }

        Config {
            port: var(PORT_VARIABLE),
            database_url: var(DATABASE_URL_VARIABLE),
            database_max_connection_count: var(DATABASE_MAX_CONNECTION_COUNT_VARIABLE),
            redis_url: var(REDIS_URL_VARIABLE),
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
        }
    }
}

fn var<T: FromStr>(name: &str) -> T {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("Missing environment variable: {}", name))
        .parse()
        .unwrap_or_else(|_| panic!("Failed to parse environment variable: {}", name))
}
