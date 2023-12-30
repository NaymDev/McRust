#[derive(Debug, PartialEq, Eq)]
pub enum State {
    HANDSHAKE,
    STATUS,
    LOGIN,
    CONFIGURATION,
}