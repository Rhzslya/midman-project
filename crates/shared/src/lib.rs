use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomInfo {
    pub room_code: String,
    pub status: String,
}
