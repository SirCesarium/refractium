use crate::commands::Commands;
use crate::display;
use refractium::core::Refractium;
use refractium::protocols::dns::Dns;
use refractium::protocols::ftp::Ftp;
use refractium::protocols::http::Http;
use refractium::protocols::https::Https;
use refractium::protocols::ssh::Ssh;
use refractium::protocols::{DynamicProtocol, ProtocolRegistry};
use refractium::types::{ProxyConfig, Transport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use tokio::net::lookup_host;
use tokio_util::sync::CancellationToken;

pub fn setup_refractium(config: &ProxyConfig, cancel_token: CancellationToken) -> Refractium {
    let (registry_tcp, routes_tcp) = setup_engine(config, &Transport::Tcp);
    let (registry_udp, routes_udp) = setup_engine(config, &Transport::Udp);

    Refractium::builder()
        .registries(registry_tcp, registry_udp)
        .routes(routes_tcp, routes_udp)
        .peek_config(config.peek_buffer_size, config.peek_timeout_ms)
        .max_connections(config.max_connections)
        .cancel_token(cancel_token)
        .build()
}

pub async fn execute_refractium(
    command: Option<Commands>,
    refractium: Arc<Refractium>,
    _cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    let addr = resolve_addr_from_str("0.0.0.0:8080").await?;

    match command {
        Some(Commands::Tcp) => {
            display::print_info("TCP", "0.0.0.0", 8080);
            refractium.run_tcp(addr).await.map_err(Into::into)
        }
        Some(Commands::Udp) => {
            display::print_info("UDP", "0.0.0.0", 8080);
            refractium.run_udp(addr).await.map_err(Into::into)
        }
        _ => {
            display::print_info("TCP + UDP", "0.0.0.0", 8080);
            refractium.run_both(addr).await.map_err(Into::into)
        }
    }
}

pub fn get_routes(config: &ProxyConfig, filter: &Transport) -> HashMap<String, Vec<String>> {
    let mut routes = HashMap::new();
    let built_in_map = [
        ("http", Transport::Tcp),
        ("https", Transport::Tcp),
        ("ssh", Transport::Tcp),
        ("ftp", Transport::Tcp),
        ("dns", Transport::Udp),
    ];

    for route in &config.protocols {
        let mut effective_transport = &route.transport;

        // If it's a built-in, enforce its native transport
        if let Some(&(name, ref native)) = built_in_map.iter().find(|(n, _)| *n == route.name) {
            if route.transport == Transport::Both {
                effective_transport = native;
            } else if route.transport != *native {
                display::print_error(&format!(
                    "Protocol mismatch: '{}' must be {:?}, but is configured as {:?}",
                    name, native, route.transport
                ));
                process::exit(1);
            }
        }

        if *effective_transport == *filter || *effective_transport == Transport::Both {
            let key = route.sni.as_ref().map_or_else(
                || route.name.clone(),
                |sni| format!("{}:{}", route.name, sni),
            );
            routes.insert(key, route.forward_to.to_vec());
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

    if matches!(filter, Transport::Tcp | Transport::Both) {
        #[cfg(feature = "proto-http")]
        registry.register(Box::new(Http));
        #[cfg(feature = "proto-https")]
        registry.register(Box::new(Https));
        #[cfg(feature = "proto-ssh")]
        registry.register(Box::new(Ssh));
        #[cfg(feature = "proto-ftp")]
        registry.register(Box::new(Ftp));
    }
    if matches!(filter, Transport::Udp | Transport::Both) {
        #[cfg(feature = "proto-dns")]
        registry.register(Box::new(Dns));
    }

    for route in &config.protocols {
        if (route.transport == *filter || route.transport == Transport::Both)
            && let Some(ref patterns) = route.patterns
        {
            registry.register(Box::new(DynamicProtocol {
                name: route.name.clone(),
                patterns: patterns.clone(),
            }));
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
