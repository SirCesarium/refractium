# Architecture and Design Overview

Refractium is built as a modular, asynchronous L4 proxy engine. Unlike standard proxies that operate on a fixed port-to-backend mapping, Refractium utilizes an identification phase to determine the protocol of an incoming stream before deciding the routing destination.

## The Peeking Mechanism

The core of Refractium's multiplexing capability is the "Peeking" phase. When a new connection is established (TCP) or a packet is received (UDP), the engine does not immediately consume the data. Instead:

1.  **Peeking**: The engine utilizes the asynchronous `peek` capability of the underlying socket. This allows the engine to look at the first few bytes of the stream without removing them from the kernel's receive buffer. The size of this buffer and the timeout for this phase are configurable via the `RefractiumBuilder`.
2.  **Identification**: The peeked data is passed to a `ProtocolRegistry`. Each registered `RefractiumProtocol` implementation analyzes the bytes to determine if they match its signature.
3.  **Routing**: Upon a successful match, the engine retrieves the associated `ProtocolRoute`. This route contains the destination backends and optional metadata like SNI (Server Name Indication).
4.  **Forwarding**: Once the destination is determined, the engine stops peeking and begins the transparent proxying phase, establishing a bidirectional asynchronous tunnel between the source and the selected backend.

## Connection Lifecycle

1.  **Ingress**: The `TcpServer` or `UdpServer` accepts a new connection/packet.
2.  **Constraint Validation**: Concurrent connection limits and per-IP limits are checked.
3.  **Protocol Probing**: The `Router` iterates through registered protocols.
4.  **Load Balancing**: The `LoadBalancer` selects an active, healthy backend from the `ForwardTarget`.
5.  **Proxying**: Data is streamed bidirectionally between the two endpoints until one side closes the connection or the `CancellationToken` is triggered.

## Thread Safety and Concurrency

Refractium is designed for high-concurrency environments:
- Protocols are stored as `Arc<dyn RefractiumProtocol>`, allowing them to be shared across multiple worker threads.
- Routing tables are protected by `RwLock` or atomic structures, enabling zero-downtime hot reloads of the routing logic.
- The `HealthMonitor` runs in the background, updating backend status without blocking the main data path.

## The L4/L7 Hybrid Advantage

Refractium is architecturally defined as an L4 (Transport Layer) proxy, but it possesses L7 (Application Layer) intelligence. This hybrid nature provides the best of both worlds:

- **L4 Performance**: By operating primarily on raw byte streams using optimized asynchronous I/O, Refractium achieves high throughput and ultra-low latency. It avoids the massive overhead of full application-layer parsing, TLS termination (unless required), and header re-serialization that traditional L7 proxies (like Nginx or HAProxy in full L7 mode) impose.
- **L7 Intelligence via Hooks**: While the data path remains focused on L4 speed, the identification phase and the Hook system provide L7-like capabilities. Hooks allow for deep packet inspection (DPI) and real-time traffic modification.
- **Efficiency**: You only pay for the L7 features you use. If no hooks are attached, the engine runs at full speed using standard asynchronous copies. If hooks are enabled, they intercept the stream for real-time processing, ensuring that the critical path remains as efficient as possible within the async runtime.

This approach allows Refractium to act as a lightweight "Application-Aware" multiplexer that scales with your infrastructure without becoming a bottleneck.
