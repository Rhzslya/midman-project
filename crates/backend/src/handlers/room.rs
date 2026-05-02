use axum::Json;
use rand::{distr::Alphanumeric, RngExt};
use shared::RoomInfo;

pub async fn create_room() -> Json<RoomInfo> {
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
