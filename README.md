# netcap

**Cross-platform HTTP/HTTPS MITM capture tool** built in Rust.

Intercept, inspect, and record HTTP/HTTPS traffic with full headers, bodies, and timestamps. Supports multiple storage backends and cross-platform deployment.

---

## Features

- **HTTP/HTTPS Capture** -- MITM proxy with automatic TLS certificate generation
- **Multiple Storage Backends** -- SQLite, JSONL, PCAP (simultaneously usable)
- **Real-time Console Logging** -- Timestamped request/response with headers and body preview
- **Domain Filtering** -- Include/exclude domains with wildcard patterns
- **Request Replay** -- Replay captured traffic from JSONL or SQLite
- **Cross-platform** -- Linux, macOS, Windows, Android, iOS
- **TUI Dashboard** -- Terminal UI for browsing captured traffic (ratatui)
- **FFI Support** -- UniFFI bindings for mobile integration

## Quick Start

### Install

```bash
# From source
cargo install --path crates/netcap-cli

# Or use the install script
curl -sSL https://raw.githubusercontent.com/tk-aria/netcap/main/scripts/setup.sh | bash
```

### Basic Usage

```bash
# Start capturing on port 8080 (default)
netcap capture

# Specify listen address and storage
netcap capture -l 127.0.0.1:9090 -s sqlite -s jsonl -o ./captures
```

### Using with curl

```bash
# HTTP traffic
curl -x http://127.0.0.1:8080 http://example.com

# HTTPS traffic (trust the generated CA)
curl -x http://127.0.0.1:8080 --cacert ./captures/netcap-ca/ca.pem https://example.com

# HTTPS traffic (skip verification)
curl -x http://127.0.0.1:8080 -k https://example.com
```

### Console Output

Captured traffic is displayed in real-time with timestamp, headers, and body:

```
[2026-03-13T16:52:20.255Z] POST http://httpbin.org/post -> 200 (432 ms)
  > Request Headers:
  >   content-type: application/json
  >   x-custom-header: test123
  > Request Body (33 bytes): {"name":"netcap","version":"0.1"}
  < Response Headers:
  <   content-type: application/json
  <   server: gunicorn/19.9.0
  < Response Body (550 bytes): {"args": {}, "data": ...}
```

## Commands

### `netcap capture` -- Start Capture

```bash
netcap capture [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-l, --listen` | Listen address | `127.0.0.1:8080` |
| `-s, --storage` | Storage backend (repeatable) | `sqlite` |
| `-o, --output-dir` | Output directory | `.` |
| `-i, --include` | Include domain filter | - |
| `-e, --exclude` | Exclude domain filter | - |

Storage types: `sqlite`, `jsonl`, `pcap`

```bash
# Capture only specific domains
netcap capture -i "*.example.com" -e "ads.example.com"

# Use multiple storage backends simultaneously
netcap capture -s sqlite -s jsonl -s pcap -o ./data
```

### `netcap cert` -- CA Certificate Management

```bash
# Generate a new CA certificate
netcap cert generate -o my-ca

# Export existing CA certificate
netcap cert export -o ca-export.pem
```

### `netcap replay` -- Replay Captured Requests

```bash
# Replay from JSONL file
netcap replay -f captures/netcap.jsonl

# Replay to a different target
netcap replay -f captures/netcap.db -t http://localhost:3000
```

## Data Recording

All captured traffic records the following data:

| Data | SQLite | JSONL | Console |
|------|--------|-------|---------|
| Timestamp | `timestamp` (RFC 3339) | `timestamp` (ISO 8601) | ISO 8601 |
| Method & URL | `method`, `url` | `method`, `uri` | Displayed |
| Request Headers | `headers_json` | `request.headers` | Displayed |
| Request Body | `body` (BLOB) | `body_base64` | Preview (200 chars) |
| Response Status | `status_code` | `response.status` | Displayed |
| Response Headers | `headers_json` | `response.headers` | Displayed |
| Response Body | `body` (BLOB) | `body_base64` | Preview (200 chars) |
| Latency | `latency_us` | `latency_ms` | Displayed |

## Architecture

```
netcap (workspace)
|-- netcap-core         Core library: proxy, TLS, capture, filtering
|-- netcap-cli          CLI application (clap)
|-- netcap-tui          Terminal UI dashboard (ratatui)
|-- netcap-ffi          FFI bindings for mobile (UniFFI)
|-- netcap-storage-sqlite   SQLite storage backend
|-- netcap-storage-jsonl    JSONL storage backend
|-- netcap-storage-pcap     PCAP storage backend
|-- netcap-storage-bigquery BigQuery storage backend
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| [hudsucker](https://crates.io/crates/hudsucker) | HTTP/HTTPS MITM proxy engine |
| [rustls](https://crates.io/crates/rustls) | TLS implementation |
| [rcgen](https://crates.io/crates/rcgen) | Dynamic certificate generation |
| [clap](https://crates.io/crates/clap) | CLI argument parsing |
| [ratatui](https://crates.io/crates/ratatui) | Terminal UI framework |
| [rusqlite](https://crates.io/crates/rusqlite) | SQLite database |

## Building

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build release
cargo build --workspace --release
```

### Mobile Builds

```bash
# Android (requires cargo-ndk + Android NDK)
./scripts/build-android.sh

# iOS (requires macOS + Xcode)
./scripts/build-ios.sh

# Generate UniFFI bindings
./scripts/generate-bindings.sh
```

## Configuration

netcap can be configured via `netcap.toml`:

```toml
[proxy]
listen_addr = "127.0.0.1:8080"
max_body_size = 10485760  # 10 MB

[storage]
type = "sqlite"
output_dir = "./captures"

[filter]
include = ["*.example.com"]
exclude = ["ads.*"]
```

## License

MIT OR Apache-2.0
