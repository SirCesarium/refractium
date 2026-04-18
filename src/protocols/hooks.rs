//! Hook system for real-time protocol traffic interception.
//!
//! Hooks allow you to inspect or mirror raw traffic as it flows through the proxy.
//! They are executed asynchronously in a dedicated task, ensuring that slow hook
//! logic does not block the main proxy data path.

use bytes::Bytes;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

/// The direction of the captured traffic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Data flowing from the client to the backend.
    Inbound,
    /// Data flowing from the backend to the client.
    Outbound,
}

/// Trait for implementing traffic interception hooks.
///
/// Implement this trait to create custom logic that reacts to every packet
/// flowing through a hooked protocol.
///
/// # Example
///
/// ```rust
/// use refractium::protocols::hooks::{ProtocolHook, HookContext, Direction};
/// use bytes::Bytes;
///
/// #[derive(Clone)]
/// struct PacketLogger;
///
/// impl ProtocolHook for PacketLogger {
///     fn name(&self) -> &'static str { "logger" }
///     fn on_packet(&self, ctx: &HookContext, dir: Direction, pkt: Bytes) {
///         println!("[{}] {:?} packet: {} bytes", ctx.session_id, dir, pkt.len());
///     }
/// }
/// ```
pub trait ProtocolHook: Send + Sync + dyn_clone::DynClone {
    /// Returns the unique name of the hook.
    fn name(&self) -> &'static str;

    /// Process a captured packet.
    ///
    /// This method is called asynchronously for every read or write operation on
    /// a hooked stream.
    fn on_packet(&self, context: &HookContext, direction: Direction, packet: Bytes);
}

/// Contextual information for a specific connection being hooked.
#[derive(Debug, Clone)]
pub struct HookContext {
    /// The remote IP address and port of the client.
    pub client_addr: SocketAddr,
    /// The name of the protocol identified by the proxy.
    pub protocol: String,
    /// A unique 64-bit identifier for the current session.
    pub session_id: u64,
}

dyn_clone::clone_trait_object!(ProtocolHook);

/// A wrapper around an asynchronous stream that dispatches data to a set of hooks.
///
/// `HookedStream` implements [`AsyncRead`] and [`AsyncWrite`], intercepting every
/// operation and sending a copy of the data to the configured [`ProtocolHook`]s.
pub struct HookedStream<S> {
    inner: S,
    direction: Direction,
    tx: mpsc::Sender<(Direction, Bytes)>,
}

impl<S> HookedStream<S> {
    /// Creates a new `HookedStream` wrapping the provided inner stream.
    ///
    /// This will spawn a dedicated background task to process the hooks for this stream.
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
