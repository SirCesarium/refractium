//! # T-Port
//! A lightweight L4 protocol multiplexer.

pub mod protocol;
pub mod proxy;

pub use protocol::{Protocol, identify};
pub use proxy::tunnel;
