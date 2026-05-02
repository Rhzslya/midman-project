use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};

pub async fn get_pool() -> Result<Pool<Postgres>, Error> {
    let url = "postgres://seira:RootPassword123@localhost:5432/midman-db";
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(60))
        .connect(url)
        .await
}
