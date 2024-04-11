use uuid::Uuid;
use crate::utils::packets::serialization::Serializable;

#[derive(Default)]
pub struct Angle {
    pub(crate) value: u8
}

#[derive(Default)]
pub struct PlayerItemListModifier {
    pub action: i32,
    pub players: Vec<Player>,
}

#[derive(Default)]
pub struct Player {
    pub uuid: Uuid,
    pub name: String,
    pub properties: Vec<Property>,
    pub gamemode: i32,
    pub ping: i32,
    pub display_name: String,
}

#[derive(Default)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub signature: String
}