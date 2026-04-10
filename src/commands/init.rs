use prisma_rs::types::Transport;

use crate::{
    config::{ServerConfig, TomlConfig, TomlRoute, TomlTarget},
    display,
};
use std::fs;

pub fn execute(path: &str) -> anyhow::Result<()> {
    let default_config = TomlConfig {
        server: ServerConfig {
            bind: "0.0.0.0".to_string(),
            port: 8080,
            peek_buffer_size: 1024,
            peek_timeout_ms: 3000,
        },
        protocols: vec![TomlRoute {
            name: "http".to_string(),
            patterns: None,
            forward_to: TomlTarget::Single("127.0.0.1:3000".to_string()),
            transport: Transport::Tcp,
        }],
        fallback_tcp: None,
        fallback_udp: None,
    };

    let toml_string = toml::to_string_pretty(&default_config)?;
    fs::write(path, toml_string)?;

    display::print_success(&format!("Configuration initialized in {path}"));

    Ok(())
}
