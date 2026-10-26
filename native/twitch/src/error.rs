use thiserror::Error;

/// Everything a Twitch call can fail with. Every failure is returned; none
/// is turned into an empty result.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TwitchError {
    /// The request did not complete (DNS, TLS, connection, timeout).
    #[error("network error: {0}")]
    Network(String),
    /// Twitch answered with a non-success HTTP status.
    #[error("HTTP status {status}")]
    Http { status: u16 },
    /// The body was not the shape we expect.
    #[error("unexpected response: {0}")]
    Decode(String),
    /// GQL answered HTTP 200 with errors we do not classify further.
    #[error("Twitch GQL errors: {}", .0.join("; "))]
    Gql(Vec<String>),
    /// Twitch wants an integrity token, which the anonymous client lacks.
    #[error("Twitch requires an integrity check")]
    IntegrityRequired,
    /// Twitch no longer knows a persisted-query hash; the hash constant for
    /// `operation` needs updating.
    #[error("Twitch retired the persisted query for {operation}")]
    QueryRetired { operation: &'static str },
    /// The stream or VOD may not be played anonymously (sub-only, blocked).
    #[error("playback is forbidden")]
    PlaybackForbidden,
    /// The channel exists but is not live.
    #[error("channel is offline")]
    Offline,
    /// The channel or video does not exist.
    #[error("not found")]
    NotFound,
}
