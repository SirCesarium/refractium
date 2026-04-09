use tokio::io::{self, copy_bidirectional};
use tokio::net::TcpStream;

/// Proxies data between two TCP streams bidirectionally.
///
/// This function sets `nodelay` on both sockets and performs a zero-copy
/// (where possible) transfer of data between the client and the backend.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// - Setting `nodelay` fails.
/// - Connecting to either stream fails.
/// - The bidirectional copy operation encounters a network error.
pub async fn proxy_tcp(mut client: TcpStream, mut backend: TcpStream) -> io::Result<()> {
    client.set_nodelay(true)?;
    backend.set_nodelay(true)?;
    copy_bidirectional(&mut client, &mut backend).await?;
    Ok(())
}
