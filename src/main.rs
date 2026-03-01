use clap::Parser;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};

#[derive(Parser, Debug)]
#[command(author, version, about = "L4 Protocol Multiplexer")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:25565")]
    listen: String,

    #[arg(short, long, default_value = "127.0.0.1:3000")]
    web: String,

    #[arg(short, long, default_value = "127.0.0.1:25567")]
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
        Err(_) => {
            if debug {
                eprintln!("Connection timed out waiting for handshake");
            }
            return Ok(());
        }
    };

    if n < 3 {
        return handle_binary(socket, mc_t, debug).await;
    }

    // check method + space to avoid collisions with mc varints
    let is_http = match &buf {
        _ if n >= 4 && (&buf[0..4] == b"GET " || &buf[0..4] == b"PUT ") => true,
        _ if n >= 5 && (&buf[0..5] == b"POST " || &buf[0..5] == b"HEAD ") => true,
        _ if n >= 6 && &buf[0..6] == b"PATCH " => true,
        _ if n >= 7 && &buf[0..7] == b"DELETE " => true,
        _ if n >= 8 && &buf[0..8] == b"OPTIONS " => true,
        _ => false,
    };

    if is_http {
        handle_web(socket, web_t, debug).await
    } else {
        handle_binary(socket, mc_t, debug).await
    }
}

async fn handle_web(
    mut socket: TcpStream,
    target_addr: String,
    debug: bool,
) -> tokio::io::Result<()> {
    if debug {
        println!("HTTP request -> redirecting to {}", target_addr);
    }

    let mut target = TcpStream::connect(target_addr).await?;
    socket.set_nodelay(true)?;
    tokio::io::copy_bidirectional(&mut socket, &mut target).await?;
    Ok(())
}

async fn handle_binary(
    mut socket: TcpStream,
    target_addr: String,
    debug: bool,
) -> tokio::io::Result<()> {
    if debug {
        println!("BINARY request -> redirecting to {}", target_addr);
    }

    let mut target = TcpStream::connect(target_addr).await?;
    socket.set_nodelay(true)?;
    tokio::io::copy_bidirectional(&mut socket, &mut target).await?;
    Ok(())
}
