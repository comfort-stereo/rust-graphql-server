use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct State {
    pub config: Config,
    pub db: PgPool,
    pub redis: ConnectionManager,
}

impl State {
    pub fn new(config: Config, db: PgPool, redis: ConnectionManager) -> Self {
        Self { config, db, redis }
    }
}
