# Out of Scope

A living list of items raised during design or implementation that were explicitly deferred, decided against, or noted as "maybe later". Kept here so ideas don't disappear into the black hole of spec files after each release.

Grouped by category. When an item from here ships in a future release, remove it from this file and note the shipping version in the CHANGELOG.

---

## Deferred — could revisit

### Check digits

- **Albania NIPT** — check letter algorithm is not publicly documented. `stdnum-js` explicitly marks it as "not understood". Ship if authoritative docs emerge.
- **Bosnia and Herzegovina JIB** — no check digit algorithm found in any accessible source; no `python-stdnum` or `stdnum-js` module exists.
- **Kosovo NUI** — newer system (~2019); no public algorithm documentation; no stdnum module.
- **Registrant-aware ISBN-13 hyphenation** — needs the ISBN registrant-prefix lookup table (large, maintained upstream). Current simple 3-1-2-5-1 fallback is fine for most uses.
- **110+ year warning for non-SE personal IDs** — Danish CPR, Finnish henkilötunnus, Norwegian fødselsnummer, Bulgarian EGN. Mechanically identical to the Swedish personnummer case in 0.18.0; just not asked for yet.
- **Partial / prefix verification** — "is this a plausible IIN / EDRPOU prefix?" for inputs shorter than the full length. UX pattern rather than an algorithm.
- **VIES live lookup** — online EU VAT validation against the official service. Requires internet request and would be architecturally distinct from the offline check-digit math.
- **Non-European tax IDs** — ASEAN, Latin America (Brazilian CPF / CNPJ are the most-requested gap), African, Middle Eastern, Australian ABN, US EIN / SSN format check, etc.

### Encoding (0.14.0)

- **Image → text decoding** (barcode/QR scanning). Would pull in `rxing` or similar heavier dependency.
- **Aztec, PDF417, MaxiCode** and other 2D formats beyond QR / DataMatrix.
- **Human-readable text under 1D barcodes** (the digits shown below an EAN-13, for example).
- **Logo overlay / colour customisation** on QR codes.
- **Multi-code image composition** (several codes on one canvas).

### Encryption

Still deferred after 0.46.0's PGP / rekey landing:

- **Hardware-backed keys** (`age-plugin-*`). Requires either an age-crate bump that exposes plugin hooks (0.11 doesn't), or re-implementing age's plugin-protocol state machine ourselves. GPG smartcards work naturally via the `gpg` subprocess when the user's keyring is already configured — no recon work needed there.
- **Mixed recipient-and-passphrase in one invocation**. age 0.11's `Encryptor::with_recipients` rejects `scrypt::Recipient` alongside X25519 recipients ("scrypt::Recipient can't be used with other recipients"). Producing a mixed-stanza header would require bypassing age's Encryptor and writing custom stanzas — a significant re-implementation. Revisit if age 0.12+ relaxes the constraint, or if there's concrete demand.

### HTTP / curl compatibility

- **`--cert-status`** — OCSP-staple check during the TLS handshake. Requires a custom `rustls::ServerCertVerifier` that inspects the staple and falls back to a network OCSP responder. Niche in practice (most deployments disable OCSP entirely in favour of short-lived certs). Revisit if a concrete need appears.
- **DER client-cert / client-key formats + encrypted PKCS#8** — Non-PEM client-cert formats and encrypted-at-rest keys. Currently rejected at load time with `openssl` conversion recipes. In-process parsing would add the `pkcs8` crate and a DER→rustls shim; shipping conversion-via-shell is the right trade-off until there's concrete demand.
- **`-w` / `--write-out` connection-phase timings** — `time_namelookup`, `time_connect`, `time_appconnect`, `time_pretransfer` currently render as `0.000000`. The accurate variables (`time_total`, `time_starttransfer`, `time_redirect`, plus every non-timing variable) work correctly. reqwest 0.12's blocking client wraps an async hyper client internally, so cleanly hooking a custom connector to record DNS/TCP/TLS phases requires either bypassing reqwest for a direct hyper + tokio stack, or waiting for upstream connector-instrumentation hooks. Revisit when either path becomes cheap.
- **`--anyauth`** — auto-select auth scheme. Security-risky (credential probing) and niche.
- **`--ntlm` / `--negotiate`** — Windows NTLM / Kerberos-SPNEGO auth. Pulls in external crates; niche for modern APIs.
- **Netscape-format cookie file** (`--cookie <file>` and `--cookie-jar <file>` in Netscape format). recon's `.db` cookiejar model is intentionally different.
- **`-w` variables outside the 22-variable subset** — `num_connects`, `proxy_ssl_verify_result`, `http_connect`, FTP-era fields. Unreachable or meaningless via reqwest.
- **`-w` `%{output{filename}}`** — redirect part of output to a specific file. Niche.
- **Interface name resolution for `--interface`** — `--interface eth0` / `en0` lookup. Current impl accepts IP literals only. Unix would need `if_nametoindex` + `getifaddrs`; Windows wants `GetAdapterAddresses`. Defer until someone asks.
- **`--engine`** — OpenSSL crypto engine selection. N/A under rustls.
- **`--dns-interface`** — bind DNS queries to a named interface. Accepted at the CLI but not yet plumbed; hickory 0.24's `NameServerConfig::bind_addr` takes a SocketAddr (IP + port), not an interface name. Socket-level `SO_BINDTODEVICE` (Linux) / `IP_BOUND_IF` (macOS) would need a custom hickory socket factory. Use `--dns-ipv4-addr` / `--dns-ipv6-addr` with the literal address as a workaround.

### curl-parity — deferred (0.50.0 sweep)

Tracked alongside `docs/curl-parity-matrix.md` for day-to-day user reference.

- **Kerberos / SPNEGO / GSS-API** — all three share the `libgssapi-krb5` dependency on Linux/macOS and Windows SSPI on Windows. Three FFI integrations is a significant cross-platform maintenance tax for a diagnostic tool. Users needing enterprise auth tend to have curl installed for exactly these cases. Revisit if concrete demand appears.
- **NTLM** — Windows-only via the `sspi` crate's FFI. Niche in modern APIs; documented as a curl gap recon doesn't try to paper over.
- **alt-svc** — RFC 7838 Alt-Svc header cache. `reqwest` has zero primitives; hand-rolling a spec-compliant cache + file persistence is ~300 lines. Low practical value for a one-shot CLI (the cache would be populated and discarded on every run). Revisit if IPv6+HTTP/3-adoption changes the calculus.
- **MultiSSL** — curl can ship with multiple TLS backends (OpenSSL + Schannel + NSS + …). Rust binaries pick one; recon picks rustls. Not a coverage gap; architectural mismatch.

### UX niggles

- **`--editor` value grabbing** — clap's `num_args = 0..=1` greedily consumes the next token, so `recon --editor https://url` treats the URL as the editor value. Documented workaround (`--editor=value`, or `--url` first); could be fixed with a smarter arg parser.

### MQTT

Still deferred after 0.45.0's power-user landing:

- **Client-certificate auth (mTLS)** — not yet in recon's HTTP surface either; unify when added.
- **Dual rustls majors in the binary** — rumqttc 0.24 pins rustls 0.22; recon's HTTPS stack uses rustls 0.23. Both coexist (~300 KB overhead). Revisit when rumqttc bumps to rustls 0.23.

---

## Out of scope by design

These are items where we've actively decided not to ship, with a reason.

### Security boundary

- **CVV / CVC validation** — the 3-4 digit card security code is cryptographically generated from PAN + expiry + issuer's secret CVK. Impossible to verify without access to the card-issuer's key material.
- **Mass scanning / credential stuffing / detection evasion tooling** — outside the scope of a reconnaissance and verification tool, regardless of how plausibly a feature could be implemented.

### Feature-mismatch

- **EIN, SSN, postal codes, phone numbers** — these have format rules but no algorithmic check digit. A format-validation feature is a different tool.

### Protocol scope

- **SMB / SMBS** — permanently deferred pending a mature pure-Rust SMB client crate. The `smb` crate is at 0.5.x and low-volume; `pavao` requires system libsmbclient (unacceptable for a cross-platform binary). Revisit when the ecosystem matures. (Note: FTP, TFTP, GOPHER, POP3, IMAP, SFTP and many others have shipped as protocol probes — this note tracks only the still-excluded remainder.)

---

## Notes on process

- When a new idea is parked during a brainstorm, add it here along with a one-line reason.
- When an item here ships, remove it and (if the version log matters) note "shipped in x.y.z" in the CHANGELOG entry rather than leaving a crossed-out line here.
- This file is deliberately not versioned in `CHANGELOG.md` — it's a working-notes file, not a release artifact.
