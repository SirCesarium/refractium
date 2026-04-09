use crate::config::Config;
use crate::display;
use std::fs;

pub fn execute(path: &str) -> anyhow::Result<()> {
    let default_config = Config::default();

    let toml_string = toml::to_string_pretty(&default_config)?;
    fs::write(path, toml_string)?;

    display::print_success(&format!("Configuration initialized in {path}"));

    Ok(())
}
