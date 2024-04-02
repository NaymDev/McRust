use super::packets::serialization::{Int};

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    HANDSHAKE,
    STATUS,
    LOGIN,
    PLAY,
}