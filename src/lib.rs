//! # Prisma-RS
//! A lightweight L4 protocol multiplexer.
#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::absolute_paths)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::missing_errors_doc)]

pub mod core;
pub mod errors;
pub mod macros;
pub mod protocol;
pub mod protocols;
pub mod proxy;

pub use protocol::{Protocol, identify};
pub use proxy::tunnel;
