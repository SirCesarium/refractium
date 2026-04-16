# Architecture and Design Overview

Refractium is built as a modular, asynchronous L4 proxy engine. Unlike standard proxies that operate on a fixed port-to-backend mapping, Refractium utilizes an identification phase to determine the protocol of an incoming stream before deciding the routing destination.

## The Peeking Mechanism

The core of Refractium's multiplexing capability is the "Peeking" phase. When a new connection is established (TCP) or a packet is received (UDP), the engine does not immediately consume the data. Instead:

1.  **Peeking**: The engine utilizes the `MSG_PEEK` flag (or an internal buffer for UDP) to look at the first few bytes of the stream. The size of this buffer and the timeout for this phase are configurable via the `RefractiumBuilder`.
2.  **Identification**: The peeked data is passed to a `ProtocolRegistry`. Each registered `RefractiumProtocol` implementation analyzes the bytes to determine if they match its signature.
3.  **Routing**: Upon a successful match, the engine retrieves the associated `ProtocolRoute`. This route contains the destination backends and optional metadata like SNI (Server Name Indication).
4.  **Splicing**: Once the destination is determined, the engine stops peeking and begins the transparent proxying phase, establishing a bidirectional tunnel between the source and the selected backend.

## Connection Lifecycle

1.  **Ingress**: The `TcpServer` or `UdpServer` accepts a new connection/packet.
2.  **Constraint Validation**: Concurrent connection limits and per-IP limits are checked.
3.  **Protocol Probing**: The `Router` iterates through registered protocols.
4.  **Load Balancing**: The `LoadBalancer` selects an active, healthy backend from the `ForwardTarget`.
5.  **Proxying**: Data is streamed between the two endpoints until one side closes the connection or the `CancellationToken` is triggered.

## Thread Safety and Concurrency

Refractium is designed for high-concurrency environments:
- Protocols are stored as `Arc<dyn RefractiumProtocol>`, allowing them to be shared across multiple worker threads.
- Routing tables are protected by `RwLock`, enabling zero-downtime hot reloads of the routing logic.
- The `HealthMonitor` runs in the background, updating backend status without blocking the main data path.
