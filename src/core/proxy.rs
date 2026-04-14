use tokio::io::{self, copy_bidirectional};
use tokio::net::TcpStream;

#[cfg(feature = "hooks")]
use crate::protocols::hooks::{Direction, HookedStream, ProtocolHook};
#[cfg(feature = "hooks")]
use std::sync::Arc;

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
pub async fn proxy_tcp(
    mut client: TcpStream,
    mut backend: TcpStream,
    #[cfg(feature = "hooks")] hooks: Vec<Arc<dyn ProtocolHook>>,
) -> io::Result<()> {
    client.set_nodelay(true)?;
    backend.set_nodelay(true)?;

    #[cfg(feature = "hooks")]
    {
        if !hooks.is_empty() {
            let mut hooked_client = HookedStream::new(client, Direction::Inbound, hooks.clone());
            let mut hooked_backend = HookedStream::new(backend, Direction::Outbound, hooks);
            copy_bidirectional(&mut hooked_client, &mut hooked_backend).await?;
            return Ok(());
        }
    }

    copy_bidirectional(&mut client, &mut backend).await?;
    Ok(())
}
