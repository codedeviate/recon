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
- **QR error-correction level tuning** (currently defaulted).
- **Logo overlay / colour customisation** on QR codes.
- **Multi-code image composition** (several codes on one canvas).

### Compression (0.13.0)

- **lz4, xz, snappy, zlib** — alternatives to the gzip/deflate/zstd/brotli/bzip2 already shipped.

### Encryption (0.15.0)

- **PGP / GPG interop** — age-format only right now.
- **Hardware-backed keys** (`age-plugin-*`).
- **Key rotation / management**.
- **Mixed recipient-and-passphrase in one invocation** — v1 takes recipients-only when both are supplied; could change to produce a header that accepts either.

### HTTP / curl compatibility

- **Additional curl flags still unimplemented** — `--tlsv1.2`, `--cacert`, `--key-type`, `--cert-status`, some others raised earlier but not currently specced.
- **`-w` / `--write-out` connection-phase timings** — `time_namelookup`, `time_connect`, `time_appconnect`, `time_pretransfer` currently render as `0.000000`. The accurate variables (`time_total`, `time_starttransfer`, `time_redirect`, plus every non-timing variable) work correctly. reqwest 0.12's blocking client wraps an async hyper client internally, so cleanly hooking a custom connector to record DNS/TCP/TLS phases requires either bypassing reqwest for a direct hyper + tokio stack, or waiting for upstream connector-instrumentation hooks. Revisit when either path becomes cheap.
- **`--anyauth`** — auto-select auth scheme. Security-risky (credential probing) and niche.
- **`--ntlm` / `--negotiate`** — Windows NTLM / Kerberos-SPNEGO auth. Pulls in external crates; niche for modern APIs.
- **Netscape-format cookie file** (`--cookie <file>` and `--cookie-jar <file>` in Netscape format). recon's `.db` cookiejar model is intentionally different.
- **`-w` variables outside the 22-variable subset** — `num_connects`, `proxy_ssl_verify_result`, `http_connect`, FTP-era fields. Unreachable or meaningless via reqwest.
- **`-w` `%{output{filename}}`** — redirect part of output to a specific file. Niche.
- **`--interface`** — bind socket to a specific local interface. Rare.
- **`--engine`** — OpenSSL crypto engine selection. N/A under rustls.
- **`--dns-servers` / `--dns-interface` / `--dns-ipv4-addr` / `--dns-ipv6-addr`** — custom DNS override.
- **`--speed-limit` / `--speed-time`** — minimum-speed abort threshold. `--limit-rate` (planned) covers typical bandwidth control.

### SMTP / SMTPS (mail delivery)

- **`smtp://` / `smtps://` protocol support** — send a test message, validate STARTTLS negotiation, exercise DKIM signing at the relay. Would complement the existing MTA-STS / TLS-RPT / SPF / DMARC / DKIM-record validation (which only inspect DNS, not the wire). Natural fit for recon's "diagnose a server" model; `lettre` crate is the standard Rust choice. Deferred pending demand — email-security DNS checks already cover most recon use cases.

### Two-source comparison

- **`recon --compare <A> <B>`** — diff two sources (URLs, files, stdin). Discussed once as "could be useful"; never specced.

### UX niggles

- **`--editor` value grabbing** — clap's `num_args = 0..=1` greedily consumes the next token, so `recon --editor https://url` treats the URL as the editor value. Documented workaround (`--editor=value`, or `--url` first); could be fixed with a smarter arg parser.

---

## Out of scope by design

These are items where we've actively decided not to ship, with a reason.

### Security boundary

- **CVV / CVC validation** — the 3-4 digit card security code is cryptographically generated from PAN + expiry + issuer's secret CVK. Impossible to verify without access to the card-issuer's key material.
- **Mass scanning / credential stuffing / detection evasion tooling** — outside the scope of a reconnaissance and verification tool, regardless of how plausibly a feature could be implemented.

### Feature-mismatch

- **EIN, SSN, postal codes, phone numbers** — these have format rules but no algorithmic check digit. A format-validation feature is a different tool.

### Protocol scope

- **Non-HTTP protocols** — FTP, SMTP, TFTP, LDAP, GOPHER, SMB, POP3, IMAP, RTSP, TELNET (beyond recon's own telnet subcommand), MQTT, DICT. recon is HTTP(S)-only; these are permanently out of scope, not deferred.

---

## Notes on process

- When a new idea is parked during a brainstorm, add it here along with a one-line reason.
- When an item here ships, remove it and (if the version log matters) note "shipped in x.y.z" in the CHANGELOG entry rather than leaving a crossed-out line here.
- This file is deliberately not versioned in `CHANGELOG.md` — it's a working-notes file, not a release artifact.
