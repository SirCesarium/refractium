//! # Refractium
//!
//! A high-performance, lightweight L4 protocol multiplexer.
//!
//! `refractium` allows you to route incoming TCP and UDP traffic to different backends
//! based on the protocol identification (magic bytes, SNI, etc.). It supports
//! dynamic protocol registration, load balancing, and health monitoring.
//!
//! ## Core Features
//! - **TCP & UDP Support**: Multiplex both stream and packet-based traffic.
//! - **Protocol Identification**: Built-in support for HTTP, HTTPS (with SNI), SSH, DNS, and FTP.
//! - **Dynamic Routing**: Add custom protocols using simple patterns or complex logic.
//! - **Load Balancing**: Distribute traffic across multiple backends with health checks.
//! - **Graceful Shutdown**: Built-in support for cancellation tokens.
//!
//! ## Feature Flags
//! Refractium uses cargo features to keep the core binary lean:
//! - `full`: Enables all features (cli, logging, protocols, watch, hooks).
//! - `protocols`: Includes all built-in protocol identifiers.
//! - `hooks`: Enables the protocol interception system.
//! - `logging`: Enables `tracing` integration.
//!
//! ## Quick Start
//! ```rust,no_run
//! use refractium::{Refractium, Http, types::{ProtocolRoute, ForwardTarget, Transport}};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 1. Define routing table
//!     let routes = vec![ProtocolRoute {
//!         protocol: Arc::new(Http),
//!         sni: None,
//!         forward_to: ForwardTarget::Single("127.0.0.1:8080".to_string()),
//!     }];
//!
//!     // 2. Build the engine
//!     let refractium = Refractium::builder()
//!         .routes(routes, Vec::new())
//!         .build()?;
//!
//!     // 3. Start the server
//!     refractium.run_tcp("0.0.0.0:80".parse()?).await?;
//!     Ok(())
//! }
//! ```

#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(missing_docs)]
#![deny(clippy::absolute_paths)]

/// Core logic for proxying and routing.
pub mod core;
/// Error types and result aliases.
pub mod errors;
/// Public macros for protocol and hook definition.
#[macro_use]
pub mod macros;
/// Protocol implementations and identification logic.
pub mod protocols;

pub use crate::core::types;
pub use crate::core::types::{ProtocolMatch, ProtocolRoute, RefractiumProtocol, Transport};
pub use crate::core::{Refractium, RefractiumBuilder};
pub use crate::errors::{RefractiumError, Result};
pub use crate::protocols::{DynamicProtocol, ProtocolRegistry};
pub use bytes;
pub use dyn_clone;
pub use heck;

#[cfg(feature = "proto-dns")]
pub use crate::protocols::dns::Dns;
#[cfg(feature = "proto-ftp")]
pub use crate::protocols::ftp::Ftp;
#[cfg(feature = "proto-http")]
pub use crate::protocols::http::Http;
#[cfg(feature = "proto-https")]
pub use crate::protocols::https::Https;
#[cfg(feature = "proto-ssh")]
pub use crate::protocols::ssh::Ssh;
