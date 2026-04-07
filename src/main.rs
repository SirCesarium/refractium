use clap::Parser;
use prisma_rs::{Protocol, identify, tunnel};
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let listener = TcpListener::bind(&args.listen).await?;

    let web_addr = args.web;
    let mc_addr = args.bin;
    let debug = args.debug;

    println!("L4 Protocol Multiplexer listening on {}", args.listen);
    println!("route HTTP traffic    => {}", web_addr);
    println!("route BINARY traffic  => {}", mc_addr);
    if debug {
        println!("debug mode: ENABLED");
    }
    println!("---------------------------------------");

    loop {
        let (socket, addr) = listener.accept().await?;
        let w_target = web_addr.clone();
        let m_target = mc_addr.clone();

        tokio::spawn(async move {
            match handle_connection(socket, w_target, m_target, debug).await {
                Err(e) if debug => eprintln!("error at {}: {}", addr, e),
                _ => (),
            }
        });
    }
}

async fn handle_connection(
    socket: TcpStream,
    web_t: String,
    mc_t: String,
    debug: bool,
) -> tokio::io::Result<()> {
    let mut buf = [0u8; 8];
    let n = match timeout(Duration::from_secs(5), socket.peek(&mut buf[..])).await {
        Ok(result) => result?,
        Err(_) => return Ok(()),
    };

    match identify(&buf[..n]) {
        Protocol::Http => {
            if debug {
                println!("HTTP request -> {}", web_t);
            }
            tunnel(socket, web_t).await
        }
        Protocol::Binary => {
            if debug {
                println!("BINARY request -> {}", mc_t);
            }
            tunnel(socket, mc_t).await
        }
    }
}
