use std::time::Duration;

/// Why a request to an exchange failed. Cloneable so it can travel in UI messages.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ExchangeError {
    #[error("network error: {0}")]
    Network(String),
    #[error("the exchange is not available from this location")]
    RestrictedLocation,
    #[error("rate limited by the exchange, retry in {retry_after:?}")]
    RateLimited { retry_after: Duration },
    #[error("exchange error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("unexpected response from the exchange: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for ExchangeError {
    fn from(error: reqwest::Error) -> Self {
        ExchangeError::Network(error.to_string())
    }
}
