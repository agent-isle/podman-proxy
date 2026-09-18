#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::indexing_slicing)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod http;
mod parse;
pub mod proxy;
mod secret_detection;
mod transport;
pub mod types;

pub use proxy::start_proxy;
pub use types::SandboxMount;
