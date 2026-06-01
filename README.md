# recon

[![GitHub](https://img.shields.io/badge/github-codedeviate%2Frecon-181717?logo=github)](https://github.com/codedeviate/recon)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?logo=opensourceinitiative)](LICENSE)
[![Rust edition 2021](<https://img.shields.io/badge/rust-2021_edition_(MSRV_1.85)-CE422B?logo=rust>)](https://www.rust-lang.org)
<br/>
[![Latest release](https://img.shields.io/github/v/release/codedeviate/recon?logo=semanticrelease&label=release&color=blue)](https://github.com/codedeviate/recon/releases)
[![crates.io](https://img.shields.io/badge/crates.io-recon--cli-fc8d62?logo=rust)](https://crates.io/crates/recon-cli)
[![Homebrew](https://img.shields.io/badge/homebrew-codedeviate%2Fcli%2Frecon-fbb040?logo=homebrew)](https://github.com/codedeviate/homebrew-cli)

A versatile network reconnaissance CLI written in Rust. Started as a curl
clone and grew into a multi-protocol investigation tool covering HTTP(S),
TLS certificate inspection, DNS, WHOIS, ping, traceroute, barcode encode/
decode, file compression and archiving, Markdown / HTML / PDF conversion,
and a full Rhai script engine that exposes every protocol probe and helper
for automation.

```sh
recon https://example.com/                  # curl-style HTTP request
recon example.com --cert                    # inspect server's TLS cert chain
recon example.com --dns A,MX,TXT            # DNS in one shot
recon --spf --dmarc --dkim example.com      # email-protection sweep
recon --script my-flow.rhai                  # Rhai scripts with full HTTP/probe access
```

## Highlights

- **40+ URL schemes**: HTTP(S), FTP(S), SCP, SFTP, TFTP, Gopher, Telnet,
  SSH, IMAP(S), POP3(S), SMTP(S), MQTT(S), Redis, Memcached, LDAP(S),
  RTSP(S), DICT, NTP, IPFS/IPNS, WS(S), TCP, UDP, file, …
- **TLS at the protocol level**: certificate inspection, CRL revocation
  checking, client cert mTLS, CA pinning, HSTS persistence.
- **Browser fingerprint impersonation** (opt-in feature, 0.77.0): mimic
  Chrome, Firefox, Safari, Edge, mobile, or OkHttp at the JA3 / JA4 / H2
  fingerprint level via BoringSSL. See [Browser fingerprint
  impersonation](#browser-fingerprint-impersonation) below.
- **Email protection sweep**: SPF, DMARC, DKIM, MTA-STS, TLS-RPT, BIMI in
  one command (`recon --spf --dmarc --dkim --mta-sts --tls-rpt --bimi`).
- **Document conversion**: Markdown → HTML, Markdown → PDF, HTML → PDF
  with cover pages, ToC, page breaks, and PDF metadata.
- **Codecs and crypto**: hash (10 algorithms), encode / decode (base64,
  base32, hex, URL, percent, …), encrypt / decrypt (age, PGP shellout),
  compress / decompress (gzip, brotli, zstd, lz4, snappy, deflate, xz,
  zlib), archive / extract (zip, tar, tar.gz, …), barcode encode / decode
  (QR, DataMatrix, Aztec, PDF417, MaxiCode, plus 1D codes).
- **Rhai script engine**: every CLI feature is also a script binding —
  `http()`, `dns()`, `ping()`, `tcp_connect()`, `mqtt_pub()`, …
  with a sticky-session `browser()` for stateful flows.
- **Curl compatibility**: most curl flags work as you'd expect (`-X`,
  `-H`, `-d`, `-L`, `-o`, `-I`, `-K`, `-u`, `-x`, `--data-binary`,
  `--retry`, `--time-cond`, `--xattr`, `-E` mTLS, `-n` netrc, …).
  See [docs/curl-parity-matrix.md](docs/curl-parity-matrix.md).

## Install

### Homebrew (macOS / Linuxbrew)

```sh
brew tap codedeviate/cli
brew install recon                # default rustls build
# or, with BoringSSL-based browser fingerprint impersonation:
brew install recon-impersonate
```

The two formulas install the same `recon` binary and conflict; pick one.

### crates.io

The crate is published as `recon-cli` (the bare `recon` name has been
parked since 2019). The installed binary is still `recon`:

```sh
cargo install recon-cli                                # default build
cargo install recon-cli --features impersonate         # with impersonation
```

### From source

```sh
git clone https://github.com/codedeviate/recon.git
cd recon
make install                      # installs to ~/.cargo/bin

# or with the impersonate feature (BoringSSL, ~5–10 MB extra binary,
# slow first build):
make install-impersonate
```

Build only without installing:

```sh
make release                      # default build (rustls-only)
make release-impersonate          # release build + browser fingerprint
                                  # impersonation feature
```

`make help` lists every target.

## Quick start

```sh
# Verbose request with header capture
recon https://api.example.com/v1/items -i

# POST JSON
recon -X POST https://api.example.com/items \
      -H 'Content-Type: application/json' \
      -d '{"name":"thing"}'

# Inspect a TLS cert chain (works on expired or self-signed certs)
recon https://example.com --cert

# Multiple DNS record types in one query
recon --dns A,AAAA,MX,TXT,DNSKEY example.com

# Email-protection aggregate report
recon --spf --dmarc --dkim --mta-sts --tls-rpt --bimi example.com

# WHOIS with two-hop registrar referral
recon --whois example.com

# Save a markdown document as PDF with ToC and cover page
recon --md-to-pdf README.md \
      --toc --gfm --doc-title 'recon README' \
      -o README.pdf

# Run a Rhai script
recon --script script/dns.rhai example.com A,MX
```

For more examples grouped by feature area:

```sh
recon --examples                  # ~60 sections of curated scenarios
recon --help <topic>              # long-form reference (e.g. tls, proxy, mqtt, jwt)
recon --flags                     # alphabetical curl-style flag index
```

### Configuration

recon reads a layered TOML config: an optional system layer
(`/etc/recon/config.toml`, or `$HOMEBREW_PREFIX/etc/recon/config.toml`
on macOS) and an optional user layer (`~/.recon/config.toml`). The
layers are deep-merged with user winning. Bootstrap your user layer
with `recon --init`; see `recon --help configuration` or the
[Configuration files section of the manual](docs/MANUAL.md#configuration-files)
for details.

## Browser fingerprint impersonation

recon 0.77.0 added an opt-in Cargo feature `impersonate` that pulls in
[`rquest`](https://crates.io/crates/rquest) (BoringSSL) plus
[`rquest-util`](https://crates.io/crates/rquest-util) so recon can mimic a
real browser's TLS+H2 fingerprint instead of its default
reqwest+rustls signature. Useful when a server uses JA3 / JA4
fingerprinting or HTTP/2 SETTINGS-frame analysis to distinguish bots from
real browsers.

```sh
make install-impersonate          # one-time, installs feature-on binary

recon --impersonate chrome_131 https://example.com/
recon --impersonate firefox_128 https://tls.peet.ws/api/all
recon --impersonate safari_ios_17.4.1 https://example.com/
```

`--ja3` / `--ja4` / `--http2-fingerprint` are reserved in the CLI for
forward-compatibility but error at runtime in v1; named profiles cover
the captcha-testing use case. See `recon --help impersonate` for the full
profile list and v1 incompatibility rules.

## Documentation

- **[docs/MANUAL.md](docs/MANUAL.md)** — the long-form user manual.
  Mirrored to `docs/MANUAL.pdf` (committed).
- **[CHANGELOG.md](CHANGELOG.md)** — every release, keep-a-changelog
  format.
- **[HISTORY.md](HISTORY.md)** — design rationale per feature: why the
  approach, what was rejected, what was deferred.
- **[OUT-OF-SCOPE.md](OUT-OF-SCOPE.md)** — what recon won't do, and why.
- **[docs/curl-parity-matrix.md](docs/curl-parity-matrix.md)** — recon
  ↔ curl `--version` feature mapping.
- **[script/README.md](script/README.md)** — Rhai script gallery,
  one focused `.rhai` per binding module.
- **`recon --help [topic]`** — built-in topic-organised help.
- **`recon --examples`** — curated runnable scenarios.

## Build matrix

```sh
make ci                # default-feature: fmt-check + clippy + test
make ci-impersonate    # ci + a parallel build/test pass with the
                       # impersonate feature (BoringSSL)
```

## Building Debian packages

Cross-build Linux `recon` binaries and `.deb` packages (amd64 + arm64) from
macOS — no Docker. Default build only (the impersonate variant is not packaged).

One-time prerequisites:

```sh
brew install zig
cargo install cargo-zigbuild cargo-deb
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
make linux-deps        # verifies the above
```

Then:

```sh
make dist              # → dist/recon_<ver>_{amd64,arm64}.deb + recon-<ver>-{x86_64,aarch64}-linux.tar.gz
make deb               # just the .deb packages
make tarball           # just the binary tarballs
make dist-clean-deb    # remove dist/
```

The `.deb` installs `recon` to `/usr/bin/recon` and depends only on `libc6`
(OpenSSL and sqlite are statically vendored for the Linux build). Inspect one
with `dpkg-deb --info <file>.deb`.

## License

MIT. Repository at https://github.com/codedeviate/recon.
