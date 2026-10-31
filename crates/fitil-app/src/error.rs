//! Errors shown to the user, in a form that can be cloned into UI messages.

use fitil_exchange::ExchangeError;
use fitil_market::MarketError;
use fitil_store::StoreError;

use crate::i18n::Translations;

#[derive(Debug, Clone)]
pub enum AppError {
    RestrictedLocation,
    RateLimited {
        seconds: u64,
    },
    Network,
    /// The exchange rejected a request; the detail is the exchange's own message.
    Exchange(String),
    /// The candle cache could not be used.
    Storage(String),
    /// What the user made, such as templates, could not be read or saved.
    UserData(String),
}

impl AppError {
    /// An error from the user's own data rather than from the cache.
    pub fn user_data(error: MarketError) -> Self {
        match error {
            MarketError::Storage(detail) => AppError::UserData(detail),
            other => other.into(),
        }
    }

    pub fn message(&self, t: &Translations) -> String {
        match self {
            AppError::RestrictedLocation => t.get("error-restricted"),
            AppError::RateLimited { seconds } => {
                t.get_with("error-rate-limited", &[("seconds", seconds.to_string())])
            }
            AppError::Network => t.get("error-network"),
            AppError::Exchange(detail) => {
                t.get_with("error-exchange", &[("detail", detail.clone())])
            }
            AppError::Storage(detail) => t.get_with("error-storage", &[("detail", detail.clone())]),
            AppError::UserData(detail) => {
                t.get_with("error-user-data", &[("detail", detail.clone())])
            }
        }
    }
}

impl From<MarketError> for AppError {
    fn from(error: MarketError) -> Self {
        match error {
            MarketError::Exchange(ExchangeError::RestrictedLocation) => {
                AppError::RestrictedLocation
            }
            MarketError::Exchange(ExchangeError::RateLimited { retry_after }) => {
                AppError::RateLimited {
                    seconds: retry_after.as_secs(),
                }
            }
            MarketError::Exchange(ExchangeError::Network(_)) => AppError::Network,
            MarketError::Exchange(ExchangeError::Api { message, .. }) => {
                AppError::Exchange(message)
            }
            MarketError::Exchange(ExchangeError::Parse(detail)) => AppError::Exchange(detail),
            MarketError::Storage(detail) => AppError::Storage(detail),
        }
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        AppError::Storage(error.to_string())
    }
}
