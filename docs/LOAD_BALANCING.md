# Load Balancing and Health Monitoring

Refractium includes a lightweight, asynchronous health monitoring system that ensures traffic is only routed to responsive backends.

## ForwardTarget

When defining a `ProtocolRoute`, you specify a `ForwardTarget`.

-   **Single**: Sends all traffic to a specific address.
-   **Multiple**: Enables round-robin load balancing across the provided list of addresses.

```rust
let target = ForwardTarget::Multiple(vec![
    "10.0.0.1:8080".to_string(),
    "10.0.0.2:8080".to_string(),
]);
```

## The Health Monitor

The `HealthMonitor` runs as a background task. It maintains the status of every backend address used in the routing table.

1.  **Passive Discovery**: The monitor starts as soon as the `Refractium` engine is built.
2.  **Active Probing**: It periodically attempts to establish a connection to each backend.
3.  **Status Propagation**: If a backend fails to respond, it is marked as "Down," and the `LoadBalancer` will automatically skip it during selection.
4.  **Automatic Recovery**: The monitor continues probing "Down" backends. Once they respond, they are marked as "Up" and reintroduced into the rotation.

## Fail-Fast and Fallbacks

Refractium implements a fail-fast strategy for routing:

-   **Unidentified Traffic**: If no protocol matches the incoming data, the engine looks for a fallback route (configured via `fallback_tcp` or `fallback_udp`). If no fallback is defined, the connection is dropped.
-   **Unhealthy Routes**: If a protocol is matched but all of its associated backends are "Down," the engine will attempt to use the global fallback route. If that also fails, the connection is terminated.

## Monitoring Performance

The health check overhead is minimal. Probes are executed independently of the data proxying path.

To generate a manual health report from your library code:

```rust
refractium.report_health().await;
```

This will print a summary of all protocols and their corresponding backend statuses to standard output.
