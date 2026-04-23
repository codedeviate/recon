# recon ↔ curl feature parity

Quick reference for how recon covers the features curl advertises in its
`--version` output. Updated alongside `OUT-OF-SCOPE.md`; if something
lands or gets explicitly declined, this file changes.

Last sweep: 0.50.0 (2026-04-23).

## Legend

- ✅ **Shipped** — feature is available via recon.
- ✅ **Always-on** — Rust / recon gets this behaviour for free by virtue
  of the toolchain or architectural choices. No user-visible flag
  needed, but the capability is present.
- ❌ **Architectural N/A** — feature doesn't map onto a single-backend
  Rust binary. Not a coverage gap.
- ⏸ **Deferred** — recon has deliberately punted. Rationale in
  [`OUT-OF-SCOPE.md`](../OUT-OF-SCOPE.md).

## Feature matrix

| curl feature | Status | Notes |
|---|---|---|
| AsynchDNS | ✅ Always-on | hickory-resolver's `TokioAsyncResolver` powers DNS resolution; see `src/dns_resolver.rs`. All DNS lookups are non-blocking at the primitive level. |
| Largefile | ✅ Always-on | Rust's native 64-bit types handle files > 2 GB on every target. |
| libz | ✅ Shipped (different impl) | Pure-Rust `flate2` crate. Identical wire behaviour; no system libz dep. Response gzip/deflate handled automatically by `--compressed` / reqwest's feature flags. |
| threadsafe | ✅ Always-on | Rust's type system enforces thread safety at compile time. No runtime flag needed. |
| MultiSSL | ❌ Architectural N/A | Rust binaries pick one TLS backend; recon picks rustls. Adding fallback (OpenSSL + rustls) would mean feature-flagged build matrices. Not a capability gap. |
| HTTP2 | ✅ Always-on | reqwest 0.12 negotiates HTTP/2 via ALPN by default. |
| HTTPS | ✅ Always-on | rustls-tls with webpki roots. |
| IPv6 | ✅ Always-on | reqwest + hickory do v6 transparently; `--dns-ipv6-addr` for local-address binding. |
| SSL | ✅ Always-on | Reported via `recon --version` (matches curl's label; actually means TLS). |
| gzip / deflate / brotli / zstd | ✅ Shipped | Pure-Rust implementations. Activate via `--compressed`. |
| Proxy (HTTP / HTTPS / SOCKS5) | ✅ 0.50.0 | `-x / --proxy URL` with scheme-based routing. See `recon --help proxy`. |
| UnixSockets | ⏸ Planned 0.51.0 | `--unix-socket /path/to/sock`. |
| HSTS | ⏸ Planned 0.52.0 | `--hsts <file>` persistent cache. |
| alt-svc | ⏸ Deferred | RFC 7838 Alt-Svc header cache. reqwest exposes no primitives; hand-rolling ~300 lines for a niche one-shot use case didn't pass the bar. See OUT-OF-SCOPE.md. |
| Kerberos / SPNEGO / GSS-API | ⏸ Deferred | All three share the libgssapi-krb5 FFI dep on Linux/macOS and Windows SSPI on Windows. Cross-platform maintenance tax significant; users needing enterprise auth tend to have curl installed. |
| NTLM | ⏸ Deferred | Windows-only via the `sspi` crate's FFI. Niche for modern APIs. |

## Glossary

Curl features not in the table are either trivially present via Rust
stdlib (stdin/stdout, file I/O, signal handling) or so curl-specific
(debug-mode, libcurl-only concepts) that mapping them to recon isn't
meaningful.
