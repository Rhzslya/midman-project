use axum::{
    routing::{get, post},
    serve, Json, Router,
};
use rand::{distr::Alphanumeric, RngExt};
use shared::RoomInfo;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/", get(|| async { "Midman Server is running" }))
        .route("/room/create", post(create_room))
        .layer(cors);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://127.0.0.1:3000");
    serve(listener, app).await.unwrap();
}

async fn create_room() -> Json<RoomInfo> {
    let room_code: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect::<String>()
        .to_uppercase();

    let room = RoomInfo {
        room_code,
        status: "OPEN".to_string(),
    };

    Json(room)
}

#[cfg(test)]
mod tests {
    use crate::create_room;

    #[test]
    fn test_hello() {
        println!("Hello, world!");
    }

    #[tokio::test]
    async fn test_create_room() {
        let room = create_room().await;
        println!("{:?}", room);
    }
}
