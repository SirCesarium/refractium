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
//! ## Quick Start
//! ```rust,no_run
//! use refractium::{Refractium, ProtocolRegistry, Http};
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut registry = ProtocolRegistry::new();
//!     registry.register(Arc::new(Http));
//!
//!     let mut routes = HashMap::new();
//!     routes.insert("Http".to_string(), vec!["127.0.0.1:8080".to_string()]);
//!
//!     let refractium = Refractium::builder()
//!         .registries(Arc::new(registry), Arc::new(ProtocolRegistry::new()))
//!         .routes(routes, HashMap::new())
//!         .build();
//!
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
/// Internal macros for protocol definition.
pub mod macros;
/// Protocol implementations and identification logic.
pub mod protocols;

pub use crate::core::types;
pub use crate::core::types::Transport;
pub use crate::core::{Refractium, RefractiumBuilder};
pub use crate::errors::{RefractiumError, Result};
pub use crate::protocols::{DynamicProtocol, ProtocolMatch, ProtocolRegistry, RefractiumProtocol};
pub use dyn_clone;
pub use bytes;

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
