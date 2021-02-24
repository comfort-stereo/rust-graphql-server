use redis::{aio::ConnectionManager, Client as RedisClient, RedisResult};
use sqlx::{postgres::PgPoolOptions, Error as SqlxError, PgPool};

use crate::config::Config;

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

pub async fn connect_to_redis(Config { redis_url, .. }: &Config) -> RedisResult<ConnectionManager> {
    ConnectionManager::new(RedisClient::open(redis_url.as_str())?).await
}
