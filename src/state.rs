use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::config::Config;

/// Global shared state for the server. This should be relatively cheap to clone and should be
/// sharable between threads.
#[derive(Clone)]
pub struct State {
    /// Server configuration.
    pub config: Config,
    /// Postgres database connection pool.
    pub db: PgPool,
    /// Redis database connection manager.
    pub redis: ConnectionManager,
}

impl State {
    /// Create a new global state object.
    pub fn new(config: Config, db: PgPool, redis: ConnectionManager) -> Self {
        Self { config, db, redis }
    }
}
