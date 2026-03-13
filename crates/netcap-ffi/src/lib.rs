pub mod error;
pub mod proxy;
pub mod types;

pub use error::FfiError;
pub use proxy::NetcapProxy;
pub use types::{FfiCaptureStats, FfiProxyConfig};

// UniFFI scaffolding - will be enabled when UDL bindings are fully integrated
// uniffi::include_scaffolding!("netcap");
