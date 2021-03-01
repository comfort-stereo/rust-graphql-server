use redis::{aio::ConnectionManager, Client as RedisClient, RedisResult};
use sqlx::{postgres::PgPoolOptions, Error as SqlxError, PgPool};

use crate::config::Config;

/// Attempt to connect to the Postgres database using the provided configuration. Connections to the
/// database are pooled.
pub async fn connect_to_db(
    Config {
        database_url,
        database_max_connection_count,
        ..
    }: &Config,
) -> Result<PgPool, SqlxError> {
    PgPoolOptions::new()
        .max_connections(*database_max_connection_count as u32)
        .connect(database_url)
        .await
}

/// Attempt to connect to the Redis database using the provided configuration.
pub async fn connect_to_redis(Config { redis_url, .. }: &Config) -> RedisResult<ConnectionManager> {
    ConnectionManager::new(RedisClient::open(redis_url.as_str())?).await
}
