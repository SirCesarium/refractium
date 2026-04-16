//! Hook system for protocol interception.

use bytes::Bytes;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

/// Traffic direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Inbound.
    Inbound,
    /// Outbound.
    Outbound,
}

/// Protocol hook trait.
pub trait ProtocolHook: Send + Sync + dyn_clone::DynClone {
    /// Returns name.
    fn name(&self) -> &'static str;
    /// Process packet.
    fn on_packet(&self, context: &HookContext, direction: Direction, packet: Bytes);
}

/// Contextual information for the current connection.
#[derive(Debug, Clone)]
pub struct HookContext {
    /// The address of the remote client.
    pub client_addr: SocketAddr,
    /// The identified protocol name.
    pub protocol: String,
    /// Unique session identifier.
    pub session_id: u64,
}

dyn_clone::clone_trait_object!(ProtocolHook);

/// Stream wrapper with hooks.
pub struct HookedStream<S> {
    inner: S,
    direction: Direction,
    tx: mpsc::Sender<(Direction, Bytes)>,
}

impl<S> HookedStream<S> {
    /// New hooked stream.
    pub fn new(
        inner: S,
        direction: Direction,
        hooks: Vec<Arc<dyn ProtocolHook>>,
        context: HookContext,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<(Direction, Bytes)>(1024);
        let context = Arc::new(context);

        tokio::spawn(async move {
            while let Some((dir, pkt)) = rx.recv().await {
                for hook in &hooks {
                    hook.on_packet(&context, dir, pkt.clone());
                }
            }
        });

        Self {
            inner,
            direction,
            tx,
        }
    }

    fn dispatch_hooks(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        let pkt = Bytes::copy_from_slice(data);
        let tx = self.tx.clone();
        let direction = self.direction;

        if let Err(mpsc::error::TrySendError::Full(_)) = tx.try_send((direction, pkt)) {
            crate::macros::refractium_warn!(
                "Hook buffer full, dropping packet for direction {:?}",
                direction
            );
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for HookedStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before_len = buf.filled().len();
        let res = Pin::new(&mut self.inner).poll_read(cx, buf);

        if matches!(&res, Poll::Ready(Ok(()))) {
            let after_len = buf.filled().len();
            if after_len > before_len {
                let data = &buf.filled()[before_len..after_len];
                self.dispatch_hooks(data);
            }
        }
        res
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for HookedStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let res = Pin::new(&mut self.inner).poll_write(cx, buf);

        if let Poll::Ready(Ok(n)) = &res
            && *n > 0
        {
            self.dispatch_hooks(&buf[..*n]);
        }
        res
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
