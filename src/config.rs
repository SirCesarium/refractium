use crate::commands::Cli;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub protocols: Vec<ProtocolRoute>,
    pub fallback_tcp: Option<String>,
    pub fallback_udp: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
    #[serde(default = "default_peek_buffer_size")]
    pub peek_buffer_size: usize,
    #[serde(default = "default_peek_timeout_ms")]
    pub peek_timeout_ms: u64,
}

const fn default_peek_buffer_size() -> usize {
    1024
}

const fn default_peek_timeout_ms() -> u64 {
    3000
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Tcp,
    Udp,
    Both,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProtocolRoute {
    pub name: String,
    pub patterns: Option<Vec<String>>,
    pub forward_to: ForwardTarget,
    #[serde(default = "default_transport")]
    pub transport: Transport,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ForwardTarget {
    Single(String),
    Multiple(Vec<String>),
}

impl ForwardTarget {
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Multiple(v) => v.clone(),
        }
    }
}

const fn default_transport() -> Transport {
    Transport::Both
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind: "0.0.0.0".to_string(),
                port: 8080,
                peek_buffer_size: default_peek_buffer_size(),
                peek_timeout_ms: default_peek_timeout_ms(),
            },
            protocols: vec![],
            fallback_tcp: None,
            fallback_udp: None,
        }
    }
}

impl Config {
    pub fn load(cli: &Cli) -> anyhow::Result<Self> {
        let mut config = if cli.no_config {
            Self::default()
        } else {
            let content = fs::read_to_string(&cli.config)?;
            toml::from_str(&content)?
        };

        config.server.bind.clone_from(&cli.bind);
        config.server.port = cli.port;

        if let Some(pb) = cli.peek_buffer {
            config.server.peek_buffer_size = pb;
        }

        if let Some(pt) = cli.peek_timeout {
            config.server.peek_timeout_ms = pt;
        }

        let mut cli_forwards: HashMap<String, Vec<String>> = HashMap::new();
        for f in &cli.forward {
            if let Some((name, addr)) = f.split_once('=') {
                cli_forwards
                    .entry(name.to_string())
                    .or_default()
                    .push(addr.to_string());
            }
        }

        for (name, addrs) in cli_forwards {
            let target = if addrs.len() == 1 {
                ForwardTarget::Single(addrs[0].clone())
            } else {
                ForwardTarget::Multiple(addrs)
            };

            if let Some(route) = config.protocols.iter_mut().find(|r| r.name == name) {
                route.forward_to = target;
            } else {
                config.protocols.push(ProtocolRoute {
                    name,
                    patterns: None,
                    forward_to: target,
                    transport: Transport::Both,
                });
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.peek_buffer_size, 1024);
    }
}
