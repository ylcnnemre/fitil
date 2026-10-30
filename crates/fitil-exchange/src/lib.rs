//! Exchange adapters for Fitil, starting with Binance.
//!
//! Only public market data is read: no API keys and no orders.

pub mod binance;
mod error;
mod rate_limit;

pub use error::ExchangeError;
pub use rate_limit::RateLimiter;

/// Makes `ring` the TLS crypto provider for this process. Safe to call repeatedly.
fn install_crypto_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // This only fails if a provider is already installed, which is just as good.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
