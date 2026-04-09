pub mod init;
pub mod run;

use clap::{Parser, Subcommand};

#[derive(Parser, Clone)]
#[command(name = "prisma", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, default_value = "0.0.0.0")]
    pub bind: String,

    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    #[arg(short, long, default_value = "prisma.toml")]
    pub config: String,

    #[arg(long)]
    pub no_config: bool,

    #[arg(long)]
    pub debug: bool,

    #[arg(short = 'F', long)]
    pub forward: Vec<String>,

    #[arg(long)]
    pub peek_buffer: Option<usize>,

    #[arg(long)]
    pub peek_timeout: Option<u64>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    Tcp,
    Udp,
    Init,
}
