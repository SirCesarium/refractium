use tokio::io::{Result, copy_bidirectional};
use tokio::net::TcpStream;

pub async fn tunnel(mut source: TcpStream, target_addr: String) -> Result<()> {
    let mut target = TcpStream::connect(target_addr).await?;
    source.set_nodelay(true)?;
    target.set_nodelay(true)?;
    copy_bidirectional(&mut source, &mut target).await?;
    Ok(())
}
