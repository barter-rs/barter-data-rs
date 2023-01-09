use barter_integration::error::SocketError;
use thiserror::Error;

/// Todo:
#[derive(Debug, Error)]
pub enum DataError {
    #[error("SocketError: {0}")]
    Socket(#[from] SocketError),

    #[error(
        "\
        InvalidSequence: first_update_id {first_update_id} does not follow on from the \
        prev_last_update_id {prev_last_update_id} \
    "
    )]
    InvalidSequence {
        prev_last_update_id: u64,
        first_update_id: u64,
    },
}

impl DataError {
    /// Todo:
    #[allow(clippy::match_like_matches_macro)]
    pub fn is_terminal(&self) -> bool {
        match self {
            DataError::InvalidSequence { .. } => true,
            _ => false,
        }
    }
}
