#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::missing_errors_doc)]

use anyhow::Context;
use clap::Parser;
use prisma_rs::core::PrismaCore;
use prisma_rs::define_protocol;
use prisma_rs::protocols::Transport;
use prisma_rs::proxy::tunnel;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};

#[derive(Parser, Debug)]
#[command(author, version, about = "L4 Protocol Multiplexer")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:80")]
    listen: String,

    #[arg(short, long, default_value = "127.0.0.1:8080")]
    web: String,

    #[arg(short, long, default_value = "127.0.0.1:9000")]
    bin: String,

    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

define_protocol!(
    HttpProto,
    "HTTP",
    Transport::Tcp,
    "web_t_placeholder",
    b"GET "
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let listener = TcpListener::bind(&args.listen)
        .await
        .context("Failed to bind TCP listener")?;

    let debug = args.debug;

    println!("L4 Protocol Multiplexer listening on {}", args.listen);
    println!("route HTTP traffic    => {}", args.web);
    println!("route BINARY traffic  => {}", args.bin);

    if debug {
        println!("debug mode: ENABLED");
    }
    println!("---------------------------------------");

    let mut core = PrismaCore::new(args.bin.clone());

    core.register(HttpProtoInst {
        target: args.web.clone(),
    });

    let core = std::sync::Arc::new(core);

    loop {
        let (socket, addr) = listener.accept().await?;
        let core_ptr = core.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, core_ptr, debug).await
                && debug
            {
                eprintln!("error at {addr}: {e}");
            }
        });
    }
}

struct HttpProtoInst {
    target: String,
}

impl prisma_rs::protocols::Protocol for HttpProtoInst {
    fn name(&self) -> &'static str {
        "HTTP"
    }
    fn transport(&self) -> Transport {
        Transport::Tcp
    }
    fn target_addr(&self) -> &str {
        &self.target
    }
    fn identify(&self, buf: &[u8]) -> bool {
        buf.starts_with(b"GET ") || buf.starts_with(b"POST ") || buf.starts_with(b"HTTP/")
    }
}

async fn handle_connection(
    socket: TcpStream,
    core: std::sync::Arc<PrismaCore>,
    debug: bool,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 16];

    let n = match timeout(Duration::from_secs(5), socket.peek(&mut buf)).await {
        Ok(result) => result.context("Peek failed")?,
        Err(_) => return Ok(()),
    };

    let target = core.resolve(&buf[..n]).to_string();

    if debug {
        println!("Routing connection to -> {target}");
    }

    tunnel(socket, target).await.context("Tunneling failed")?;

    Ok(())
}
