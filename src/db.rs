use std::time::Duration;

use async_std::task;
use redis::{aio::ConnectionManager, Client as RedisClient, RedisResult};
use sqlx::{postgres::PgPoolOptions, Error as SqlxError, PgPool};
use tide::log;

use crate::config::Config;

const MAX_CONNECTION_RETRIES: u64 = 20;
const RETRY_POLLING_INTERVAL_SECONDS: u64 = 3;

/// Attempt to connect to the Postgres database using the provided configuration. Connections to the
/// database are pooled.
pub async fn connect_to_db(
    Config {
        database_url,
        database_max_connection_count,
        ..
    }: &Config,
) -> Result<PgPool, SqlxError> {
    let mut retries = 0;
    loop {
        match PgPoolOptions::new()
            .max_connections(*database_max_connection_count as u32)
            .connect(database_url)
            .await
        {
            Ok(db) => break Ok(db),
            Err(error) => {
                if retries == MAX_CONNECTION_RETRIES {
                    break Err(error);
                }

                log::warn!("Failed to connect to Postgres database, retrying...");
                task::sleep(Duration::from_secs(RETRY_POLLING_INTERVAL_SECONDS)).await;
                retries += 1;
            }
        }
    }
}

/// Attempt to connect to the Redis database using the provided configuration.
pub async fn connect_to_redis(Config { redis_url, .. }: &Config) -> RedisResult<ConnectionManager> {
    let mut retries = 0;
    loop {
        match ConnectionManager::new(RedisClient::open(redis_url.as_str())?).await {
            Ok(redis) => break Ok(redis),
            Err(error) => {
                if retries == MAX_CONNECTION_RETRIES {
                    break Err(error);
                }

                log::warn!("Failed to connect to Redis database, retrying...");
                task::sleep(Duration::from_secs(RETRY_POLLING_INTERVAL_SECONDS)).await;
                retries += 1;
            }
        }
    }
}
