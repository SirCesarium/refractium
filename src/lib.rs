//! # Prisma-RS
//!
//! A high-performance, lightweight L4 protocol multiplexer.
//!
//! `prisma-rs` allows you to route incoming TCP and UDP traffic to different backends
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
//! use prisma_rs::core::Prisma;
//! use prisma_rs::protocols::ProtocolRegistry;
//! use prisma_rs::protocols::http::Http;
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut registry = ProtocolRegistry::new();
//!     registry.register(Box::new(Http));
//!
//!     let mut routes = HashMap::new();
//!     routes.insert("Http".to_string(), vec!["127.0.0.1:8080".to_string()]);
//!
//!     let prisma = Prisma::builder()
//!         .registries(Arc::new(registry), Arc::new(ProtocolRegistry::new()))
//!         .routes(routes, HashMap::new())
//!         .build();
//!
//!     prisma.run_tcp("0.0.0.0:80".parse()?).await?;
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
pub use crate::core::{Prisma, PrismaBuilder};
pub use crate::errors::{PrismaError, Result};
