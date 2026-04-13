use crate::commands::Cli;
use refractium::core::types::{ForwardTarget, ProtocolRoute, ProxyConfig, Transport};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;

/// Internal TOML representation for server settings.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TomlConfig {
    pub server: ServerConfig,
    pub protocols: Vec<TomlRoute>,
    pub fallback_tcp: Option<String>,
    pub fallback_udp: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
    #[serde(
        default = "default_peek_buffer",
        skip_serializing_if = "is_default_buffer"
    )]
    pub peek_buffer_size: usize,
    #[serde(
        default = "default_peek_timeout",
        skip_serializing_if = "is_default_timeout"
    )]
    pub peek_timeout_ms: u64,
    #[serde(
        default = "default_max_connections",
        skip_serializing_if = "is_default_connections"
    )]
    pub max_connections: usize,
    #[serde(
        default = "default_max_conns_per_ip",
        skip_serializing_if = "is_default_max_conns_per_ip"
    )]
    pub max_connections_per_ip: usize,
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TomlRoute {
    #[serde(deserialize_with = "lowercase_string")]
    pub name: String,
    pub sni: Option<String>,
    pub patterns: Option<Vec<String>>,
    pub forward_to: TomlTarget,
    #[serde(
        default = "default_transport",
        skip_serializing_if = "is_default_transport"
    )]
    pub transport: Transport,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum TomlTarget {
    Single(String),
    Multiple(Vec<String>),
}

const fn default_peek_buffer() -> usize {
    1024
}
const fn default_peek_timeout() -> u64 {
    3000
}
const fn default_max_connections() -> usize {
    10000
}
const fn default_max_conns_per_ip() -> usize {
    50
}
const fn default_hot_reload() -> bool {
    true
}
const fn default_transport() -> Transport {
    Transport::Both
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_default_buffer(sz: &usize) -> bool {
    *sz == default_peek_buffer()
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_default_timeout(ms: &u64) -> bool {
    *ms == default_peek_timeout()
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_default_connections(sz: &usize) -> bool {
    *sz == default_max_connections()
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_default_max_conns_per_ip(sz: &usize) -> bool {
    *sz == default_max_conns_per_ip()
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_transport(t: &Transport) -> bool {
    *t == default_transport()
}

impl Default for TomlConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind: "0.0.0.0".to_string(),
                port: 8080,
                peek_buffer_size: default_peek_buffer(),
                peek_timeout_ms: default_peek_timeout(),
                max_connections: default_max_connections(),
                max_connections_per_ip: default_max_conns_per_ip(),
                hot_reload: default_hot_reload(),
            },
            protocols: vec![],
            fallback_tcp: None,
            fallback_udp: None,
        }
    }
}

impl TomlConfig {
    fn try_load(path: &str) -> anyhow::Result<Option<Self>> {
        match fs::read_to_string(path) {
            Ok(c) => Ok(Some(toml::from_str(&c)?)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Loads configuration by merging TOML file data with CLI overrides.
    pub fn load_config(cli: &Cli) -> anyhow::Result<ProxyConfig> {
        let mut base = Self::try_load(&cli.config)?.unwrap_or_default();

        base.server.bind.clone_from(&cli.bind);
        base.server.port = cli.port;

        if let Some(pb) = cli.peek_buffer {
            base.server.peek_buffer_size = pb;
        }
        if let Some(pt) = cli.peek_timeout {
            base.server.peek_timeout_ms = pt;
        }
        if let Some(mc) = cli.max_connections {
            base.server.max_connections = mc;
        }

        if cli.no_hot_reload {
            base.server.hot_reload = false;
        }

        let mut cli_overrides: HashMap<String, Vec<String>> = HashMap::new();
        for f in &cli.forward {
            if let Some((name, addr)) = f.split_once('=') {
                cli_overrides
                    .entry(name.to_lowercase())
                    .or_default()
                    .push(addr.into());
            }
        }

        for (name, addrs) in cli_overrides {
            let target = if addrs.len() == 1 {
                TomlTarget::Single(addrs[0].clone())
            } else {
                TomlTarget::Multiple(addrs)
            };

            if let Some(route) = base.protocols.iter_mut().find(|r| r.name == name) {
                route.forward_to = target;
            } else {
                base.protocols.push(TomlRoute {
                    name,
                    sni: None,
                    patterns: None,
                    forward_to: target,
                    transport: Transport::Both,
                });
            }
        }

        base.validate();

        Ok(base.into_proxy_config())
    }

    fn validate(&self) {
        if self.server.peek_timeout_ms > 5000 {
            tracing::warn!(
                "SECURITY: peek_timeout_ms is set to {}ms. High values make the server vulnerable to Slowloris attacks.",
                self.server.peek_timeout_ms
            );
        }

        if self.server.max_connections_per_ip > 200 {
            tracing::warn!(
                "SECURITY: max_connections_per_ip is very high ({}). Consider lowering it to prevent DoS from single sources.",
                self.server.max_connections_per_ip
            );
        }

        if self.server.max_connections_per_ip >= self.server.max_connections
            && self.server.max_connections > 0
        {
            tracing::warn!(
                "SECURITY: max_connections_per_ip is equal or greater than total max_connections. Single IP can saturate the whole server."
            );
        }
    }

    fn into_proxy_config(self) -> ProxyConfig {
        ProxyConfig {
            bind: self.server.bind,
            port: self.server.port,
            peek_buffer_size: self.server.peek_buffer_size,
            peek_timeout_ms: self.server.peek_timeout_ms,
            max_connections: self.server.max_connections,
            max_connections_per_ip: self.server.max_connections_per_ip,
            hot_reload: self.server.hot_reload,
            fallback_tcp: self.fallback_tcp,
            fallback_udp: self.fallback_udp,
            protocols: self
                .protocols
                .into_iter()
                .map(|r| ProtocolRoute {
                    name: r.name,
                    sni: r.sni,
                    patterns: r.patterns,
                    transport: r.transport,
                    forward_to: match r.forward_to {
                        TomlTarget::Single(s) => ForwardTarget::Single(s),
                        TomlTarget::Multiple(v) => ForwardTarget::Multiple(v),
                    },
                })
                .collect(),
        }
    }
}

fn lowercase_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_defaults() -> anyhow::Result<()> {
        let config = TomlConfig::default();
        let serialized = toml::to_string_pretty(&config)?;

        assert!(
            !serialized.contains("peek_buffer_size"),
            "Default values should be skipped in serialization"
        );

        Ok(())
    }

    #[test]
    fn test_try_load_non_existent_file() -> anyhow::Result<()> {
        let result = TomlConfig::try_load("non_existent_refractium.toml")?;
        assert!(result.is_none());
        Ok(())
    }
}
