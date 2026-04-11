#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::absolute_paths)]

mod commands;
mod config;
mod display;

use crate::commands::{Cli, Commands, init, run};
use crate::config::TomlConfig;
use clap::Parser;
use prisma_rs::types::Transport;
use std::process;
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt;

#[cfg(feature = "watch")]
use notify::{Event, RecursiveMode, Watcher};
#[cfg(feature = "watch")]
use std::path::Path;
#[cfg(feature = "watch")]
use tokio::sync::mpsc;

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt::init();
    let cli = Cli::parse();

    if matches!(cli.command, Some(Commands::Init)) {
        return init::execute(&cli.config);
    }

    display::print_banner();

    let config = match TomlConfig::load_config(&cli) {
        Ok(c) => c,
        Err(e) => {
            display::print_error(&format!("Failed to load configuration: {e}"));
            process::exit(1);
        }
    };

    if config.protocols.is_empty() && config.fallback_tcp.is_none() && config.fallback_udp.is_none()
    {
        display::print_config_guide();
        process::exit(1);
    }

    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();

    let ctrl_c = signal::ctrl_c();
    let terminate = async {
        #[cfg(unix)]
        {
            use std::io;

            let mut sig = signal::unix::signal(signal::unix::SignalKind::terminate())?;
            sig.recv().await;
            Ok::<(), io::Error>(())
        }
        #[cfg(not(unix))]
        {
            std::future::pending::<()>().await;
            Ok::<(), io::Error>(())
        }
    };

    tokio::spawn(async move {
        tokio::select! {
            _ = ctrl_c => {
                display::print_success("Shutdown signal received (Ctrl+C)");
            }
            _ = terminate => {
                display::print_success("Termination signal received");
            }
        }
        token_clone.cancel();
    });

    let prisma = run::setup_prisma(&config, cancel_token.clone());
    let prisma_arc = Arc::new(prisma);
    let prisma_reload = Arc::clone(&prisma_arc);
    let cli_reload = cli.clone();

    if config.hot_reload {
        #[cfg(feature = "watch")]
        {
            let (tx, mut rx) = mpsc::channel(1);
            let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
                if let Ok(event) = res
                    && event.kind.is_modify()
                {
                    let _ = tx.blocking_send(());
                }
            })?;

            let config_path = cli_reload.config.clone();
            if Path::new(&config_path).exists() {
                watcher.watch(Path::new(&config_path), RecursiveMode::NonRecursive)?;

                let cli_watch = cli_reload.clone();
                let prisma_watch = Arc::clone(&prisma_reload);

                tokio::spawn(async move {
                    while rx.recv().await.is_some() {
                        display::print_success("Changes detected in configuration file");
                        if let Ok(new_config) = TomlConfig::load_config(&cli_watch) {
                            let tcp = run::get_routes(&new_config, &Transport::Tcp);
                            let udp = run::get_routes(&new_config, &Transport::Udp);
                            prisma_watch.reload_routes(tcp, udp).await;
                            display::print_success("Configuration hot-reloaded successfully");
                        }
                    }
                });
            }
        }

        #[cfg(unix)]
        {
            let cli_sig = cli_reload.clone();
            let prisma_sig = Arc::clone(&prisma_reload);
            tokio::spawn(async move {
                if let Ok(mut stream) = signal::unix::signal(signal::unix::SignalKind::hangup()) {
                    while stream.recv().await.is_some() {
                        display::print_success("SIGHUP received: Reloading configuration...");
                        if let Ok(new_config) = TomlConfig::load_config(&cli_sig) {
                            let tcp = run::get_routes(&new_config, &Transport::Tcp);
                            let udp = run::get_routes(&new_config, &Transport::Udp);
                            prisma_sig.reload_routes(tcp, udp).await;
                            display::print_success("Configuration reloaded via signal");
                        }
                    }
                }
            });
        }
    } else {
        display::print_success("Hot reload is disabled");
    }

    if let Err(e) = run::execute_prisma(cli.command, prisma_arc, cancel_token).await {
        display::print_error(&format!("Engine Error: {e}"));
    }

    Ok(())
}
