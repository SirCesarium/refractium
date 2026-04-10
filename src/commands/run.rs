use crate::commands::Commands;
use crate::display;
use prisma_rs::core::Prisma;
use prisma_rs::protocols::dns::Dns;
use prisma_rs::protocols::ftp::Ftp;
use prisma_rs::protocols::http::Http;
use prisma_rs::protocols::https::Https;
use prisma_rs::protocols::ssh::Ssh;
use prisma_rs::protocols::{DynamicProtocol, ProtocolRegistry};
use prisma_rs::types::{ProxyConfig, Transport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use tokio::net::lookup_host;
use tokio_util::sync::CancellationToken;

pub fn setup_prisma(config: &ProxyConfig, cancel_token: CancellationToken) -> Prisma {
    let (registry_tcp, routes_tcp) = setup_engine(config, &Transport::Tcp);
    let (registry_udp, routes_udp) = setup_engine(config, &Transport::Udp);

    Prisma::builder()
        .registries(registry_tcp, registry_udp)
        .routes(routes_tcp, routes_udp)
        .peek_config(config.peek_buffer_size, config.peek_timeout_ms)
        .cancel_token(cancel_token)
        .build()
}

pub async fn execute_prisma(
    command: Option<Commands>,
    prisma: Arc<Prisma>,
    _cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    let addr = resolve_addr_from_str("0.0.0.0:8080").await?;

    match command {
        Some(Commands::Tcp) => {
            display::print_info("TCP", "0.0.0.0", 8080);
            prisma.run_tcp(addr).await.map_err(Into::into)
        }
        Some(Commands::Udp) => {
            display::print_info("UDP", "0.0.0.0", 8080);
            prisma.run_udp(addr).await.map_err(Into::into)
        }
        _ => {
            display::print_info("TCP + UDP", "0.0.0.0", 8080);
            prisma.run_both(addr).await.map_err(Into::into)
        }
    }
}

pub fn get_routes(config: &ProxyConfig, filter: &Transport) -> HashMap<String, Vec<String>> {
    let mut routes = HashMap::new();
    for route in &config.protocols {
        if route.transport == *filter || route.transport == Transport::Both {
            routes.insert(route.name.clone(), route.forward_to.to_vec());
        }
    }
    let fallback = match filter {
        Transport::Tcp => config.fallback_tcp.as_ref(),
        Transport::Udp => config.fallback_udp.as_ref(),
        Transport::Both => None,
    };
    if let Some(fb) = fallback {
        routes.insert("fallback".to_string(), vec![fb.clone()]);
    }
    routes
}

fn setup_engine(
    config: &ProxyConfig,
    filter: &Transport,
) -> (Arc<ProtocolRegistry>, HashMap<String, Vec<String>>) {
    let mut registry = ProtocolRegistry::new();
    let built_ins = ["http", "https", "ssh", "dns", "ftp"];

    if matches!(filter, Transport::Tcp | Transport::Both) {
        registry.register(Box::new(Http));
        registry.register(Box::new(Https));
        registry.register(Box::new(Ssh));
        registry.register(Box::new(Ftp));
    }

    if matches!(filter, Transport::Udp | Transport::Both) {
        registry.register(Box::new(Dns));
    }

    for route in &config.protocols {
        if route.transport == *filter || route.transport == Transport::Both {
            if let Some(ref patterns) = route.patterns {
                registry.register(Box::new(DynamicProtocol {
                    name: route.name.clone(),
                    patterns: patterns.clone(),
                }));
            } else if !built_ins.contains(&route.name.as_str()) {
                #[cfg(feature = "logging")]
                tracing::warn!("Protocol '{}' ignored: no patterns", route.name);
            }
        }
    }

    (Arc::new(registry), get_routes(config, filter))
}

async fn resolve_addr_from_str(addr_str: &str) -> anyhow::Result<SocketAddr> {
    match lookup_host(addr_str).await {
        Ok(mut addrs) => addrs
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not resolve any address for {addr_str}")),
        Err(e) => {
            display::print_resolve_error(addr_str, &e.to_string());
            process::exit(1);
        }
    }
}
