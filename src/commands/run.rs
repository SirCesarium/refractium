use crate::commands::Commands;
use crate::display;
use refractium::core::Refractium;
use refractium::protocols::DynamicProtocol;
use refractium::protocols::dns::Dns;
use refractium::protocols::ftp::Ftp;
use refractium::protocols::http::Http;
use refractium::protocols::https::Https;
use refractium::protocols::ssh::Ssh;
use refractium::types::{
    ForwardTarget, ProtocolMatch, ProtocolRoute, ProxyConfig, RefractiumProtocol, TomlRouteData,
    Transport,
};
use std::process;
use std::sync::Arc;
use tokio::net::lookup_host;
use tokio_util::sync::CancellationToken;

use std::net::SocketAddr;

use refractium::errors;

/// Sets up the Refractium engine based on the provided configuration.
pub fn setup_refractium(
    config: &ProxyConfig,
    cancel_token: CancellationToken,
) -> errors::Result<Refractium> {
    let routes_tcp = setup_engine(config, Transport::Tcp)?;
    let routes_udp = setup_engine(config, Transport::Udp)?;
    Refractium::builder()
        .routes(routes_tcp, routes_udp)
        .peek_config(config.peek_buffer_size, config.peek_timeout_ms)
        .max_connections(config.max_connections)
        .max_connections_per_ip(config.max_connections_per_ip)
        .cancel_token(cancel_token)
        .build()
}

/// Executes the Refractium engine with the specified command.
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

/// Returns the routing table for the specified transport filter.
pub fn get_routes(config: &ProxyConfig, filter: Transport) -> errors::Result<Vec<ProtocolRoute>> {
    setup_engine(config, filter)
}

fn setup_engine(config: &ProxyConfig, filter: Transport) -> errors::Result<Vec<ProtocolRoute>> {
    let mut routes = Vec::new();
    for route in &config.protocols {
        if route.transport == filter || route.transport == Transport::Both {
            routes.push(ProtocolRoute {
                protocol: build_protocol(route)?,
                sni: route.sni.clone(),
                forward_to: route.forward_to.clone(),
            });
        }
    }
    add_fallback(config, filter, &mut routes);
    Ok(routes)
}

fn build_protocol(route: &TomlRouteData) -> errors::Result<Arc<dyn RefractiumProtocol>> {
    if let Some(ref patterns) = route.patterns {
        return Ok(Arc::new(DynamicProtocol {
            name: route.name.clone(),
            patterns: patterns.clone(),
        }));
    }
    match route.name.as_str() {
        "http" => Ok(Arc::new(Http)),
        "https" => Ok(Arc::new(Https)),
        "ssh" => Ok(Arc::new(Ssh)),
        "ftp" => Ok(Arc::new(Ftp)),
        "dns" => Ok(Arc::new(Dns)),
        _ => Err(errors::RefractiumError::ConfigError(format!(
            "Unknown protocol: '{}'. Built-in or patterns required.",
            route.name
        ))),
    }
}

fn add_fallback(config: &ProxyConfig, filter: Transport, routes: &mut Vec<ProtocolRoute>) {
    let fb = match filter {
        Transport::Tcp => config.fallback_tcp.as_ref(),
        Transport::Udp => config.fallback_udp.as_ref(),
        Transport::Both => None,
    };
    if let Some(target) = fb {
        routes.push(ProtocolRoute {
            protocol: Arc::new(FallbackProtocol),
            sni: None,
            forward_to: ForwardTarget::Single(target.clone()),
        });
    }
}

#[derive(Clone)]
struct FallbackProtocol;
impl RefractiumProtocol for FallbackProtocol {
    fn name(&self) -> &'static str {
        "fallback"
    }
    fn identify(self: Arc<Self>, _data: &[u8]) -> Option<ProtocolMatch> {
        None
    }
    fn transport(&self) -> Transport {
        Transport::Both
    }
}

async fn resolve_addr_from_str(addr_str: &str) -> anyhow::Result<SocketAddr> {
    match lookup_host(addr_str).await {
        Ok(mut addrs) => addrs
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not resolve {addr_str}")),
        Err(e) => {
            display::print_resolve_error(addr_str, &e.to_string());
            process::exit(1);
        }
    }
}
