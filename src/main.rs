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
use refractium::core::Refractium;
use refractium::types::{ProxyConfig, Transport};
use std::path::Path;
use std::process;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt;

#[cfg(feature = "watch")]
use notify::{Event, RecursiveMode, Watcher};
#[cfg(feature = "watch")]
use tokio::sync::mpsc;

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
    setup_shutdown_signal(cancel_token.clone());

    let refractium = Arc::new(run::setup_refractium(&config, cancel_token.clone()));

    // Wait a bit for initial health checks to complete before reporting
    time::sleep(Duration::from_millis(500)).await;
    refractium.report_health().await;

    // Setup hot reload and keep the watcher alive
    let _watcher = setup_hot_reload(&config, &cli, Arc::clone(&refractium));

    if let Err(e) = run::execute_refractium(cli.command, refractium, cancel_token).await {
        display::print_error(&format!("Engine Error: {e}"));
    }

    Ok(())
}

fn setup_shutdown_signal(token: CancellationToken) {
    tokio::spawn(async move {
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

        tokio::select! {
            _ = ctrl_c => display::print_success("Shutdown signal received (Ctrl+C)"),
            _ = terminate => display::print_success("Termination signal received"),
        }
        token.cancel();
    });
}

#[cfg(feature = "watch")]
fn setup_hot_reload(
    config: &ProxyConfig,
    cli: &Cli,
    refractium: Arc<Refractium>,
) -> Option<Box<dyn Watcher>> {
    if !config.hot_reload {
        display::print_success("Hot reload is disabled");
        return None;
    }

    let watcher = setup_file_watcher(cli, Arc::clone(&refractium));
    setup_signal_reload(cli, refractium);

    watcher
}

#[cfg(not(feature = "watch"))]
fn setup_hot_reload(config: &ProxyConfig, cli: &Cli, refractium: Arc<Refractium>) -> Option<bool> {
    if !config.hot_reload {
        display::print_success("Hot reload is disabled");
        return None;
    }
    setup_signal_reload(cli, refractium);
    None
}

#[cfg(feature = "watch")]
fn setup_file_watcher(cli: &Cli, refractium: Arc<Refractium>) -> Option<Box<dyn Watcher>> {
    let (tx, mut rx) = mpsc::channel(1);
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_ok_and(|e| e.kind.is_modify() || e.kind.is_create()) {
            let _ = tx.blocking_send(());
        }
    })
    .ok()?;

    let config_path = cli.config.clone();
    let path = Path::new(&config_path);

    if path.exists() {
        watcher.watch(path, RecursiveMode::NonRecursive).ok()?;

        let cli_watch = cli.clone();
        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                // Increased debounce to avoid duplicate reloads
                time::sleep(Duration::from_millis(500)).await;

                // Clear any extra messages in the channel during sleep
                while rx.try_recv().is_ok() {}

                display::print_success("Changes detected in configuration file");
                if let Ok(new_config) = TomlConfig::load_config(&cli_watch) {
                    let tcp = run::get_routes(&new_config, &Transport::Tcp);
                    let udp = run::get_routes(&new_config, &Transport::Udp);
                    refractium.reload_routes(tcp, udp).await;
                    display::print_success("Configuration hot-reloaded successfully");

                    // Wait for new checks
                    time::sleep(Duration::from_millis(500)).await;
                    refractium.report_health().await;
                }
            }
        });
        return Some(Box::new(watcher));
    }
    None
}

fn setup_signal_reload(cli: &Cli, refractium: Arc<Refractium>) {
    #[cfg(unix)]
    {
        let cli_sig = cli.clone();
        tokio::spawn(async move {
            if let Ok(mut stream) = signal::unix::signal(signal::unix::SignalKind::hangup()) {
                while stream.recv().await.is_some() {
                    display::print_success("SIGHUP received: Reloading configuration...");
                    if let Ok(new_config) = TomlConfig::load_config(&cli_sig) {
                        let tcp = run::get_routes(&new_config, &Transport::Tcp);
                        let udp = run::get_routes(&new_config, &Transport::Udp);
                        refractium.reload_routes(tcp, udp).await;
                        display::print_success("Configuration reloaded via signal");

                        time::sleep(Duration::from_millis(500)).await;
                        refractium.report_health().await;
                    }
                }
            }
        });
    }
}
