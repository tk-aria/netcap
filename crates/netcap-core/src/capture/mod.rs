pub mod body;
pub mod exchange;

use crate::capture::exchange::CapturedExchange;

/// Trait for handling captured HTTP exchanges.
pub trait CaptureHandler: Send + Sync + 'static {
    fn on_exchange(&self, exchange: &CapturedExchange);
}
