//! Errors shown to the user, in a form that can be cloned into UI messages.

use fitil_exchange::ExchangeError;
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
    Storage(String),
}

impl AppError {
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
        }
    }
}

impl From<ExchangeError> for AppError {
    fn from(error: ExchangeError) -> Self {
        match error {
            ExchangeError::RestrictedLocation => AppError::RestrictedLocation,
            ExchangeError::RateLimited { retry_after } => AppError::RateLimited {
                seconds: retry_after.as_secs(),
            },
            ExchangeError::Network(_) => AppError::Network,
            ExchangeError::Api { message, .. } => AppError::Exchange(message),
            ExchangeError::Parse(detail) => AppError::Exchange(detail),
        }
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        AppError::Storage(error.to_string())
    }
}
