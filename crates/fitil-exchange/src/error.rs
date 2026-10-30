use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ExchangeError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("the exchange is not available from this location (HTTP 451)")]
    RestrictedLocation,
    #[error("rate limited by the exchange, retry in {retry_after:?}")]
    RateLimited { retry_after: Duration },
    #[error("exchange error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("unexpected response from the exchange: {0}")]
    Parse(String),
}
