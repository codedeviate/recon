# Out of Scope & Wishlist

A living list of items raised during design, implementation, or feature sweeps
that are either explicitly deferred, decided against, or noted as "maybe
later". Also doubles as a wishlist — items under "Waiting" are things worth
building once someone explicitly asks. Kept here so ideas don't disappear
into the black hole of spec files after each release.

Organized into four buckets by reason for non-inclusion. When an item ships, remove it from this file and note the shipping version in the CHANGELOG entry rather than leaving a crossed-out line here.

- **Waiting** — can be done; nobody's asked for it.
- **Deferred** — possible to implement; actively put off (scope/complexity trade-off or waiting on a concrete use case).
- **Not yet supported** — blocked by upstream / ecosystem maturity; may ship when the blocker clears.
- **Out of scope** — fundamentally can't be implemented, architecturally mismatched, or intentionally declined by policy.

---

## Waiting — can be done, not asked for

### Check digits

- **110+ year warning for non-SE personal IDs** — Danish CPR, Finnish henkilötunnus, Norwegian fødselsnummer, Bulgarian EGN. Mechanically identical to the Swedish personnummer case in 0.18.0; just not asked for yet.
- **Partial / prefix verification** — "is this a plausible IIN / EDRPOU prefix?" for inputs shorter than the full length. UX pattern rather than an algorithm.
- **Non-European tax IDs** — ASEAN, Latin America (Brazilian CPF / CNPJ are the most-requested gap), African, Middle Eastern, Australian ABN, US EIN / SSN format check, etc.

### Encoding

- **Human-readable text under 1D barcodes** (the digits shown below an EAN-13, for example). ~80 LOC with `ab_glyph` for PNG; an `<text>` element for SVG.
- **Multi-barcode scanning in one image** — `rxing::helpers::detect_multiple_in_file` exists; not yet wired. Revisit if users report multi-code source images.
- **`--encode-hints` (rxing encode_with_hints)** — ECI options, Aztec compact-vs-full, PDF417 error-correction level. The API exists; user-facing flag surface isn't designed yet.

### HTTP / curl compatibility

- **Interface name resolution for `--interface`** — `--interface eth0` / `en0` lookup. Current impl accepts IP literals only. Unix would need `if_nametoindex` + `getifaddrs`; Windows wants `GetAdapterAddresses`. Defer until someone asks.

### Script engine

- **ICMP raw-socket send/recv primitives** — `ping()` already covers reachability checks; arbitrary ICMP type/code send + recv is niche. Requires raw sockets (`CAP_NET_RAW` on Linux, root on macOS for non-DGRAM types). Revisit when users ask for specific traffic-generation or monitoring use cases.

### Document conversions

- **Other markup → PDF** — reStructuredText, AsciiDoc, Org. Each would need its own parser crate. Revisit per concrete ask.
- **PDF metadata beyond title** — author, subject, keywords.

### MQTT

- **Client-certificate auth (mTLS)** — now that recon ships `--client-cert` / `--client-key` (0.54.0), plumbing the same opts through to `rumqttc` is straightforward. Just not asked for yet.

### wget features (things curl — and therefore recon — doesn't have)

Catalog of wget-unique capabilities that could land in recon if asked. Most
cluster around recursive mirroring; a few are independently useful
standalone wins. Not prioritised against each other — just documented.

**Standalone wins** (each could ship on its own without the others):

- **`-i, --input-file <PATH>`** — batch-fetch URLs from a file (one per
  line; `#` comments; `-` reads from stdin).
  - Pro: trivial to implement on top of the existing request pipeline
    (loop over the list, reuse `client::execute` per URL); pairs
    naturally with `--compare` for mass diffing and with
    `--md-to-pdf` for converting a batch of URLs. ~100 LOC.
  - Con: none meaningful. Honest answer: should probably be in the next
    minor release regardless of wget parity.

- **`-N, --timestamping`** — send `If-Modified-Since` based on local
  file's mtime; skip download if the server returns 304.
  - Pro: saves bandwidth on reruns; makes a scripted mirror-refresh
    idempotent. Handy for CI-style "re-pull if changed" workflows.
  - Con: requires consistent local-file resolution (pair with `-O`
    or `--output-path`); FTP case needs its own code path (FTP MDTM
    command). HTTP-only version would ship first, FTP as a follow-up.

- **`--spider`** — check that URLs resolve / respond 2xx without
  downloading bodies. Pair with `-i` for a bulk link checker.
  - Pro: natural fit for a diagnostic tool; easy implementation on
    top of HEAD + status-code check; the batch case becomes a drop-in
    replacement for `linkchecker` / `lychee` minus their sugar.
  - Con: edge cases around servers that reject HEAD with 405 (need
    to fall back to GET + early disconnect); exit-code conventions
    need thought (exit on first failure? summary at end? per-line
    status lines?).

- **`-c, --continue`** — resume a partial download.
  - Pro: curl's `-C -` does the same but requires users to know about
    the dash. wget's form ("continue from where we left off") is more
    discoverable.
  - Con: largely curl-parity territory; recon can already do it via
    `-C -`. Adding `--continue` would be a pure alias. Low effort,
    low signal.

- **`--http1.0`** — force HTTP/1.0 (vs 1.1).
  - Pro: occasionally useful against old embedded HTTP servers that
    misbehave on 1.1.
  - Con: reqwest's connection API doesn't expose 1.0 directly (it
    defaults to 1.1); pinning 1.0 would need a custom hyper connector.
    More work than the feature warrants unless someone hits a concrete
    interop case.

**Recursive / mirror cluster** (largely all-or-nothing; most wget
features assume recursion is on the table). The whole cluster is one
feature area; shipping a subset leaves the behaviour feeling half-done.

- **`-r, --recursive`** + **`-l, --level <N>`** — recursive fetch
  following links in HTML (or FTP listings).
  - Pro: big user-visible capability; enables `-m`, `-p`, `--convert-
    links`, etc. Positions recon as a viable wget-alternative for
    archival work.
  - Con: substantial scope. Needs an HTML link extractor (`scraper` +
    `html5ever`), URL canonicalisation + loop detection + depth
    limits, robots.txt handling, per-host rate limiting, a download
    queue, and a sane default filesystem layout. Rough estimate:
    800–1200 LOC + associated tests. Whole arc across 2–3 releases.

- **`-m, --mirror`** — convenience alias for `-r -N -l inf
  --no-remove-listing`. Depends on recursive.
  - Pro: one-flag mirror command is what most users actually want.
  - Con: depends on the whole recursive cluster landing first.

- **`-p, --page-requisites`** — fetch everything a single page needs
  to render (images, CSS, JS, icons).
  - Pro: smaller scope than full recursion; produces a reproducible
    offline snapshot of a single page, which is what most folks
    actually want when they reach for `wget -r`.
  - Con: still needs HTML parsing for `<img>`, `<link>`, `<script>`,
    `<source>`, `<video>`. Without `--convert-links` the output isn't
    locally browsable — the feature feels incomplete without its
    partner flag.

- **`-k, --convert-links`** — rewrite absolute links in downloaded
  HTML to local relative paths after mirror/page-requisites finishes.
  - Pro: turns `-p` / `-r` output into something you can open in a
    browser offline. Completes the mirror story.
  - Con: only meaningful paired with recursive or page-requisites.
    HTML rewriting has edge cases (links inside `<script>`, CSS
    `url(…)`, inline styles, data URIs).

- **Accept/reject filters** (`-A <LIST>`, `-R <LIST>`,
  `--accept-regex`, `--reject-regex`) — filter by file extension or
  regex during recursion.
  - Pro: practical scope control for a mirror (skip PDFs, skip
    images).
  - Con: recursion-dependent. Regex variants add pcre-style depen-
    dency considerations.

- **`-D <DOMAINS>`, `-H, --span-hosts`, `--exclude-domains`** —
  restrict or expand the host set during recursion.
  - Pro: critical safety rail for mirrors (don't accidentally recurse
    into Twitter / CDN domains).
  - Con: recursion-dependent.

- **`-np, --no-parent`** — during recursion, don't ascend above the
  starting directory.
  - Pro: common safety rail.
  - Con: recursion-dependent.

- **`--cut-dirs <N>`, `-nd`, `-nH`** — flatten the local filesystem
  layout (drop the host dir, drop N leading path components).
  - Pro: makes the downloaded tree readable.
  - Con: recursion-dependent cosmetics.

- **`-Q, --quota <BYTES>`** — cap total download size in recursion.
  - Pro: safety fuse; avoids runaway mirrors.
  - Con: recursion-dependent.

- **`-w, --wait <SECS>`, `--random-wait`** — politeness delay between
  recursive requests.
  - Pro: avoids triggering rate-limits; lets a long mirror run
    without getting 429'd. `--wait` is clearly benign.
  - Con: `--random-wait` has anti-bot-detection connotations that
    lightly conflict with recon's stance in OUT-OF-SCOPE's "Security
    boundary" section ("not a detection-evasion tool"). `--wait` on
    its own is fine; `--random-wait` probably isn't.

- **`-t, --tries <N>`, `--retry-connrefused`** — retry config for
  transient failures during recursion.
  - Pro: keeps mirrors resilient on flaky networks.
  - Con: recon already has `--retry` / `--retry-max-time` from curl
    parity. Wget's knobs largely duplicate these; add only the truly
    distinct ones (e.g. `--retry-connrefused` for the specific-error
    case).

- **`-b, --background`** — detach and log to a file.
  - Pro: long-running mirrors without tying up a terminal.
  - Con: easy to get from the shell (`nohup`, `&`, systemd-run);
    adding first-class support just means reinventing what every
    shell already provides. Defer unless asked.

**Assessment.** If "wget parity" ever becomes a goal, the natural
phasing is:
1. Land the standalone wins independently (`-i`, `-N`, `--spider`, `-c`).
2. Decide whether the recursive cluster is worth building or whether
   `wget` / `httrack` / `lychee` already own that workflow well enough.
3. If recursive lands, ship it as its own 3-release arc (primitives →
   filters → mirror convenience flags + link rewriting).

Realistically `-i`, `-N`, and `--spider` are the highest-value items
per LOC and would noticeably improve recon's batch-diagnostic story.
The recursive cluster is a big enough project that it probably
deserves its own spec + plan when someone actually needs it.

---

## Deferred — put off, path is known

### Check digits

- **Registrant-aware ISBN-13 hyphenation** — needs the ISBN registrant-prefix lookup table (large, maintained upstream). Current simple 3-1-2-5-1 fallback is fine for most uses.
- **VIES live lookup** — online EU VAT validation against the official service. Requires internet request and would be architecturally distinct from the offline check-digit math.

### Encoding

- **Logo overlay / colour customisation** on QR codes — fiddly UX surface; postpone until concrete demand shapes the flag set.
- **Multi-code image composition** (several codes on one canvas) — same reason.

### HTTP / curl compatibility

- **`--cert-status`** — OCSP-staple check during the TLS handshake. Requires a custom `rustls::ServerCertVerifier` that inspects the staple and falls back to a network OCSP responder. Niche in practice (most deployments disable OCSP entirely in favour of short-lived certs). Revisit if a concrete need appears.
- **DER client-cert / client-key formats + encrypted PKCS#8** — Non-PEM client-cert formats and encrypted-at-rest keys. Currently rejected at load time with `openssl` conversion recipes. In-process parsing would add the `pkcs8` crate and a DER→rustls shim; shipping conversion-via-shell is the right trade-off until there's concrete demand.
- **`--anyauth`** — auto-select auth scheme. Security-risky (credential probing) and niche.
- **`--ntlm` / `--negotiate`** — Windows NTLM / Kerberos-SPNEGO auth. Pulls in external crates; niche for modern APIs.
- **`-w` `%{output{filename}}`** — redirect part of output to a specific file. Niche.

### curl-parity — deferred (0.50.0 sweep)

Tracked alongside `docs/curl-parity-matrix.md` for day-to-day user reference.

- **Kerberos / SPNEGO / GSS-API** — all three share the `libgssapi-krb5` dependency on Linux/macOS and Windows SSPI on Windows. Three FFI integrations is a significant cross-platform maintenance tax for a diagnostic tool. Users needing enterprise auth tend to have curl installed for exactly these cases. Revisit if concrete demand appears.
- **NTLM** — Windows-only via the `sspi` crate's FFI. Niche in modern APIs; documented as a curl gap recon doesn't try to paper over.
- **alt-svc** — RFC 7838 Alt-Svc header cache. `reqwest` has zero primitives; hand-rolling a spec-compliant cache + file persistence is ~300 lines. Low practical value for a one-shot CLI (the cache would be populated and discarded on every run). Revisit if IPv6+HTTP/3-adoption changes the calculus.

### Document conversions

- **typst-based md→PDF alternative** — Chrome-free path for markdown → PDF via a hand-rolled md→typst translator + the `typst` crate embedded. Would add ~15–25 MB to the release binary and require non-trivial translator logic. Revisit if users explicitly ask for Chrome-free PDF generation.
- **Custom page sizes / margins / orientations** — agent-browser's `pdf` subcommand's flag surface dictates what's feasible. Punt until real demand shapes the knobs.

### UX niggles

- **`--editor` value grabbing** — clap's `num_args = 0..=1` greedily consumes the next token, so `recon --editor https://url` treats the URL as the editor value. Documented workaround (`--editor=value`, or `--url` first); could be fixed with a smarter arg parser.

---

## Not yet supported — blocked on upstream / ecosystem

### Check digits

- **Albania NIPT** — check letter algorithm is not publicly documented. `stdnum-js` explicitly marks it as "not understood". Ship if authoritative docs emerge.
- **Bosnia and Herzegovina JIB** — no check digit algorithm found in any accessible source; no `python-stdnum` or `stdnum-js` module exists.
- **Kosovo NUI** — newer system (~2019); no public algorithm documentation; no stdnum module.

### Encoding

- **MaxiCode encoding** — no pure-Rust encoder exists. rxing (ZXing port) decodes MaxiCode but ships no encoder. Revisit when someone writes one or if shelling out to `dmtx-utils` / `zint` becomes acceptable. (Decoding already works via `--decode` and `rxing`.)

### Encryption

Still deferred after 0.46.0's PGP / rekey landing:

- **Hardware-backed keys** (`age-plugin-*`). Requires either an age-crate bump that exposes plugin hooks (0.11 doesn't), or re-implementing age's plugin-protocol state machine ourselves. GPG smartcards work naturally via the `gpg` subprocess when the user's keyring is already configured — no recon work needed there.
- **Mixed recipient-and-passphrase in one invocation**. age 0.11's `Encryptor::with_recipients` rejects `scrypt::Recipient` alongside X25519 recipients ("scrypt::Recipient can't be used with other recipients"). Producing a mixed-stanza header would require bypassing age's Encryptor and writing custom stanzas — a significant re-implementation. Revisit if age 0.12+ relaxes the constraint.

### HTTP / curl compatibility

- **`-w` / `--write-out` connection-phase timings** — `time_namelookup`, `time_connect`, `time_appconnect`, `time_pretransfer` currently render as `0.000000`. The accurate variables (`time_total`, `time_starttransfer`, `time_redirect`, plus every non-timing variable) work correctly. reqwest 0.12's blocking client wraps an async hyper client internally, so cleanly hooking a custom connector to record DNS/TCP/TLS phases requires either bypassing reqwest for a direct hyper + tokio stack, or waiting for upstream connector-instrumentation hooks. Revisit when either path becomes cheap.
- **`--dns-interface`** — bind DNS queries to a named interface. Accepted at the CLI but not yet plumbed; hickory 0.24's `NameServerConfig::bind_addr` takes a SocketAddr (IP + port), not an interface name. Socket-level `SO_BINDTODEVICE` (Linux) / `IP_BOUND_IF` (macOS) would need a custom hickory socket factory. Use `--dns-ipv4-addr` / `--dns-ipv6-addr` with the literal address as a workaround.

### Document conversions

- **Pure-Rust HTML+CSS → PDF renderer** — `servo`/`blitz` exist but aren't packaged as an embeddable crate yet. `typst` is pure-Rust and has `#outline()` for linkable TOC, but does NOT accept HTML as input (its HTML support is output-only). Revisit if either path matures.

### MQTT

- **Dual rustls majors in the binary** — rumqttc 0.24 pins rustls 0.22; recon's HTTPS stack uses rustls 0.23. Both coexist (~300 KB overhead). Revisit when rumqttc bumps to rustls 0.23.

### Protocol scope

- **SMB / SMBS** — pending a mature pure-Rust SMB client crate. The `smb` crate is at 0.5.x and low-volume; `pavao` requires system libsmbclient (unacceptable for a cross-platform binary). Revisit when the ecosystem matures. (FTP, TFTP, GOPHER, POP3, IMAP, SFTP and many others have shipped as protocol probes — this note tracks only the still-excluded remainder.)

---

## Out of scope — can't / won't

### Security boundary

- **CVV / CVC validation** — the 3-4 digit card security code is cryptographically generated from PAN + expiry + issuer's secret CVK. Impossible to verify without access to the card-issuer's key material.
- **Mass scanning / credential stuffing / detection evasion tooling** — outside the scope of a reconnaissance and verification tool, regardless of how plausibly a feature could be implemented.

### Feature mismatch

- **EIN, SSN, postal codes, phone numbers** — these have format rules but no algorithmic check digit. A format-validation feature is a different tool.

### Architectural mismatch

- **MultiSSL** — curl can ship with multiple TLS backends (OpenSSL + Schannel + NSS + …). Rust binaries pick one; recon picks rustls. Not a coverage gap; recon deliberately picks one backend.
- **`--engine`** — OpenSSL crypto engine selection. N/A under rustls.
- **CLI server flags** (`recon --listen 0.0.0.0:8080`) — server workflows are always multi-step (accept → per-conn handler); scripts are the right layer. Quick HTTP serving is already covered by the pre-built `recon --serve`.
- **Netscape-format cookie file** (`--cookie <file>` and `--cookie-jar <file>` in Netscape format). recon's `.db` cookiejar model is intentionally different; there's no path where supporting both makes sense.
- **`-w` variables outside the 22-variable subset** — `num_connects`, `proxy_ssl_verify_result`, `http_connect`, FTP-era fields. Unreachable or meaningless via reqwest; listing them would imply support we can't give.

---

## Notes on process

- When a new idea is parked during a brainstorm, add it here under the most honest of the four buckets + a one-line reason.
- When an item here ships, remove it and note "shipped in x.y.z" in the CHANGELOG entry rather than leaving a crossed-out line here.
- Items can move between buckets as the world changes. When ecosystem maturity unblocks a "Not yet supported" item it graduates to "Waiting"; when a "Waiting" item picks up enough scope weight to merit punting, it moves to "Deferred".
- This file is deliberately not versioned in `CHANGELOG.md` — it's a working-notes file, not a release artifact.
