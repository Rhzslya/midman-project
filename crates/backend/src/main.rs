mod db;
mod handlers;

use axum::{
    routing::{get, post},
    serve, Router,
};

use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use handlers::room::create_room;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::permissive();

    db::get_pool().await.expect("Database connection failed!");
    println!("Database connection success!");

    let app = Router::new()
        .route("/", get(|| async { "Midman Server is running" }))
        .route("/room/create", post(create_room))
        .layer(cors);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://127.0.0.1:3000");
    serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {

    use crate::db::get_pool;
    use crate::handlers::room::create_room;

    use sqlx::Error;

    #[test]
    fn test_hello() {
        println!("Hello, world!");
    }

    #[tokio::test]
    async fn test_create_room() {
        let room = create_room().await;
        println!("{:?}", room);
    }

    // 3. Test for connection to database
    #[tokio::test]
    async fn test_connection_only() -> Result<(), Error> {
        let pool = get_pool().await?;

        let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;

        assert_eq!(row.0, 1);
        println!("Success connect to database!");

        Ok(())
    }

    // 4. Test for inserting data into database
    #[tokio::test]
    async fn test_execute() -> Result<(), Error> {
        let pool = get_pool().await?;

        sqlx::query("INSERT INTO users (id, name, email, password) VALUES($1, $2, $3, $4)")
            .bind(1)
            .bind("Seira")
            .bind("Developer")
            .bind("Rust")
            .execute(&pool)
            .await?;

        println!("Success insert data into database!");
        Ok(())
    }
}
