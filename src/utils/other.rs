use super::packets::serialization::{Serializeable, Int};

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    HANDSHAKE,
    STATUS,
    LOGIN,
    PLAY,
}