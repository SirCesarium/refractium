# Security Policy

## Security Philosophy

`refractium` is designed as a security-first networking primitive. By operating at the transport layer with a "peek-and-proxy" approach, the server minimizes the attack surface common in Layer 7 proxies. 

### Core Guarantees

- **Memory Safety:** Implementation is 100% Rust. The project enforces a strict no-`unsafe` policy to eliminate buffer overflows and dangling pointers.
- **Panic Denial:** CI pipelines include mandatory Clippy checks with `-D warnings` to prevent unhandled exceptions that could lead to Denial of Service (DoS).
- **Resource Constraints:** Built-in mechanisms to prevent resource exhaustion, including configurable peek timeouts and connection limits.

## Reporting a Vulnerability

If you discover a security vulnerability within `refractium`, please do not open a public issue. Follow the process below:

1. **Contact**: Please refer to the contact information available on the [project maintainer's GitHub profile](https://github.com/SirCesarium)..
2. **Details:** Include a description of the vulnerability, steps to reproduce, and a potential Proof of Concept (PoC).
3. **Response:** You will receive an acknowledgment within 48 hours. We aim to provide a fix or a mitigation plan within 7 days.

## Supported Versions

Only the latest stable release receives security updates. Users are encouraged to stay on the most recent version to benefit from the latest dependency patches and security hardening.

| Version | Supported |
| :--- | :---: |
| v1.x.x | ✅ |
| < v1.0.0 | ❌ |

## Mitigation of Network Attacks

`refractium` includes specific configurations to defend against common network-level vectors:

### Denial of Service (DoS)
- **Max Connections:** Configurable via `max_connections` to prevent memory exhaustion from massive connection spikes.
- **Per-IP Rate Limiting:** The `max_connections_per_ip` setting mitigates flooding from single sources.
- **Peek Timeouts:** Connections that remain idle during the identification phase are forcefully closed after `peek_timeout_ms` to prevent Slowloris attacks.

### Fuzzing and Malformed Traffic
The proxy does not parse protocol-specific payloads (like HTTP headers or SSH handshakes) into complex structures. It uses byte-pattern matching, making it resilient against vulnerabilities in high-level parsers.

## Supply Chain Security

We implement rigorous automated checks to ensure the integrity of the project:

- **Automated Dependency Updates:** Dependabot monitors `cargo`, `docker`, and `github-actions` weekly to ensure all components are up to date.
- **Continuous Integration:** Every commit is vetted through a CI suite that includes linting and compilation checks.
- **Binary Integrity:** Releases are built using automated GitHub Actions. Docker images are published to GHCR with specific version tags to ensure immutability.

## Pentest Suite

The repository contains a specialized testing suite under the `pentest/` directory. This suite is used to verify the server's resilience against:
- Garbage data floods.
- Malformed protocol handshakes.
- Persistent Slowloris stress tests.

To run the security suite locally using Docker Compose:

```bash
cd pentest
docker compose up --build --abort-on-container-exit --attach tester
```