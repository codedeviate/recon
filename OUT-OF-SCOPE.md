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

- **Partial / prefix verification** — "is this a plausible IIN / EDRPOU prefix?" for inputs shorter than the full length. UX pattern rather than an algorithm.
- **ASEAN / African / Middle Eastern tax IDs** — beyond the Latin-American + Australian + Mexican set shipped in 0.61.0. Add per concrete request.

### Encoding

- **PNG HRT** — 0.61.0 shipped HRT for ASCII + SVG output; PNG HRT is deferred pending font bundling. `ab_glyph` + a permissive TTF (~50–100 KB compiled) is the path; picking a font + rasterization positioning wasn't worth it in this release.
- **`--encode-hints` (rxing encode_with_hints)** — ECI options, Aztec compact-vs-full, PDF417 error-correction level. The API exists; user-facing flag surface isn't designed yet.

### Script engine

- **ICMP raw-socket send/recv primitives** — `ping()` already covers reachability checks; arbitrary ICMP type/code send + recv is niche. Requires raw sockets (`CAP_NET_RAW` on Linux, root on macOS for non-DGRAM types). Revisit when users ask for specific traffic-generation or monitoring use cases.

### Document conversions

- **Other markup → PDF** — reStructuredText, AsciiDoc, Org. Each would need its own parser crate. Revisit per concrete ask.
- **PDF metadata beyond title** — author, subject, keywords.

### Additional curl flags (`curl --help all` sweep)

A full walk through `curl --help all` against recon's flag set found
~200 curl flags not yet present. Items **already documented**
elsewhere in this file are omitted (alt-svc, anyauth, aws-sigv4,
cert-status, delegation, digest, engine, haproxy-*, krb, negotiate,
netrc*, ntlm, pinnedpubkey, proxy-{cert,key,ssl,gssapi}-*,
socks5-gssapi-*, sslv2/3, ssl-{allow-beast,auto-client-cert,
no-revoke,revoke-best-effort}, tlsauthtype/password/user,
cert-status, DER formats).

Grouped by theme. Pros / cons are spelled out for the high-value
items (likely next features if wishlist pressure arrives); the long
tail is listed compactly so the inventory stays complete.

**HTTP version pinning** (high value for interop testing):

- `-0, --http1.0`, `--http1.1`, `--http2`, `--http2-prior-knowledge`, `--http3`, `--http3-only`
  - Pro: force a specific version; invaluable when debugging HTTP/2 push,
    HTTP/3 QUIC, or legacy servers. Most flags are thin wrappers around
    reqwest's `http1_only` / `http2_prior_knowledge` builders.
  - Con: HTTP/3 needs reqwest's QUIC feature (disabled in recon); not
    free.

**Form uploads** (high value; curl parity gap):

- `-F, --form <NAME=VALUE>`, `--form-string <NAME=STRING>`, `--form-escape`
  - Pro: recon has multipart via the script `http()` opts map but NOT
    as CLI flags. Parity closes a common "why can't I do this from
    the command line?" gap.
  - Con: `-F` has a rich mini-language (`name=@file`, `name=@file;type=…`,
    `name=<file` for content from file, `name=<@-` for stdin); faithful
    reproduction is ~200 LOC.

**Conditional requests** (high value for polling):

- `-z, --time-cond <TIME|FILE>` — `If-Modified-Since` from an absolute
  date or local file's mtime.
  - Pro: saves bandwidth on reruns; pairs with `-O` for idempotent
    mirrors. Already listed on the wget side (`-N`); curl's version is
    more flexible (date strings vs file mtime).
  - Con: ~50 LOC including RFC-822 date parsing.

- `--etag-compare <FILE>` + `--etag-save <FILE>` — ETag-based conditional.
  - Pro: complements time-cond for servers that care about ETag.
  - Con: requires persistent state (the ETag file).

**Byte-range requests**:

- `-r, --range <RANGE>` — byte-range fetch. `0-1023`, `2048-`, `-500`.
  - Pro: essential for partial-content testing + resume support.
  - Con: low effort, high value. Probably ships the same release as
    `--max-filesize`.

- `--max-filesize <BYTES>` — cap download size. Server reports
  Content-Length; abort if larger.
  - Pro: safety fuse for runaway downloads (especially paired with
    a future `-i / --input-file` from the wget list).
  - Con: only useful when Content-Length is honest.

**Output control extras**:

- `--output-dir <DIR>` — prefix for `-o` / `-O`.
- `--remote-name-all` — apply `-O` to every URL in a multi-URL invocation.
- `--remove-on-error` — delete the partial output file on failure.
- `--create-file-mode <MODE>` — chmod octal for files `-o` creates.
- `-N, --no-buffer` — unbuffered stdout (useful when piping into
  another tool that wants bytes now).
- `--no-clobber` — don't overwrite existing output.
- `-D, --dump-header <FILE>` — save response headers to a file.
- `--stderr <FILE>` — redirect stderr to a file.
- `-#, --progress-bar` — bar-style progress vs the default percent meter.
- `--no-progress-meter`, `--styled-output`, `--suppress-connect-headers`

  - Pro: every item is a minor quality-of-life tweak. Collectively
    they close ~20 user papercuts.
  - Con: each is small alone; worth bundling into a single "output
    quality-of-life" release once a few get requested.

**Retry / rate**:

- `--retry <N>`, `--retry-all-errors`, `--retry-connrefused`,
  `--retry-delay <SECS>`, `--retry-max-time <SECS>` — retry loop for
  transient failures. Curl's model is self-contained; reqwest
  doesn't ship retry primitives.
  - Pro: essential for flaky networks; pairs with CI use cases.
  - Con: recon would need to build a retry layer around `client::execute`
    (~100 LOC + backoff logic).

- `--rate <MAX>` — request rate limit per interval (e.g. `2/s`).
  - Pro: useful when batching multiple URLs without tripping rate limits.
  - Con: only meaningful with multi-URL or `-i` input-file modes.

**Protocol restriction**:

- `--proto <PROTOCOLS>`, `--proto-default <PROTO>`, `--proto-redir <PROTOCOLS>`
  - Pro: security hardening (reject `-L` redirects to unexpected
    schemes). Curl's syntax (`=https,-ftp` etc.) is well-established.
  - Con: state-machine to evaluate the filter expression; ~80 LOC
    including parser.

- `-:, --next` — separator between URL-specific flag sets in a
  single invocation. `curl URL1 -H 'X: a' -: URL2 -H 'Y: b'`.
  - Pro: one-shot multi-request without scripting.
  - Con: substantial clap restructure — the positional-vs-flag pairing
    would need an argv-splitter pre-pass similar to how `--script`
    args are handled today.

**URL surface**:

- `--url-query <DATA>` — append query params (URL-encodes like
  `--data-urlencode`).
- `--request-target <PATH>` — override the request-target in the
  first request line.
- `--path-as-is` — don't collapse `..` / `.` in URLs.
- `-g, --globoff` — disable curl's `{a,b,c}` / `[1-10]` URL globbing.
  (recon doesn't glob, so this would be a no-op — skip unless we
  add globbing.)
- `--disallow-username-in-url` — security hardening.

**Parallel transfers**:

- `-Z, --parallel`, `--parallel-max <N>`, `--parallel-immediate`
  - Pro: first-class parallel fetching; complements a future `-i`
    input-file feature.
  - Con: needs a work-queue, progress aggregation, per-stream output
    routing. Non-trivial.

**Auth shortcuts**:

- `-n, --netrc`, `--netrc-file <FILE>`, `--netrc-optional` — ~/.netrc.
  - Pro: big user-quality-of-life feature; standard Unix convention.
    Many CI setups already have a netrc.
  - Con: ~80 LOC including parser and URL-match logic. Low risk.

- `--oauth2-bearer <TOKEN>` — sets `Authorization: Bearer TOKEN`.
  - Pro: curl-parity sugar; saves `-H "Authorization: Bearer $TOK"`.
  - Con: trivial (~10 LOC) but redundant with -H.

- `--digest` — HTTP Digest auth.
  - Pro: some legacy enterprise servers still use it.
  - Con: reqwest 0.12 has no Digest support; would require a custom
    401-challenge layer. ~150 LOC.

**Upload variants**:

- `-a, --append` — append to remote instead of overwrite
  (FTP / SFTP).
- `--crlf` — convert LF to CRLF during upload.
- `-T, --upload-file -` — upload from stdin (recon has `-T` but
  accepts a path; check stdin path).

**Connection tuning**:

- `--connect-to <HOST:PORT:TARGET:PORT>` — override connection target
  per-host (like `--resolve` but address-level).
- `--tcp-fastopen`, `--tcp-nodelay` — TCP tuning.
- `--keepalive-time <SECS>`, `--no-keepalive` — TCP keepalive.
- `--local-port <RANGE>` — pick source port from a range.
- `--happy-eyeballs-timeout-ms <MS>` — IPv6→IPv4 fallback timing.
- `--no-alpn`, `--no-npn`, `--no-sessionid` — TLS feature disables
  for misbehaving servers.

**TLS tuning** (niche but legitimate):

- `--ciphers <LIST>`, `--tls13-ciphers <LIST>` — custom cipher lists.
- `--tls-max <VERSION>` — upper bound.
- `--curves <LIST>` — allowed ECDH curves.
- `--crlfile <PATH>` — TLS CRL.
- `--ca-native`, `--capath <DIR>` — alternate trust stores.
- `--ssl`, `--ssl-reqd` — soft/hard TLS requirement for FTP/SMTP/etc.
- `--false-start` — TLS False Start.
- `--tlsv1`, `--tlsv1.0`, `--tlsv1.1` — force specific old versions.
- `--tr-encoding` — request Transfer-Encoding compression.

**DNS-over-HTTPS**:

- `--doh-url <URL>`, `--doh-insecure`, `--doh-cert-status`
  - Pro: privacy + censorship-resilience; would pair with `--dns-servers`.
  - Con: hickory-resolver has no DoH yet; would need hickory 0.25 or
    a side-car DoH client.

**FTP** (listed in full since recon's FTP probe binding exists):

- `--ftp-account`, `--ftp-alternative-to-user`, `--ftp-create-dirs`,
  `--ftp-method`, `--ftp-pasv`, `-P, --ftp-port`, `--ftp-pret`,
  `--ftp-skip-pasv-ip`, `--ftp-ssl-ccc`, `--ftp-ssl-ccc-mode`,
  `--ftp-ssl-control`, `-Q, --quote`, `--disable-epsv`, `--disable-eprt`,
  `-l, --list-only`, `--tftp-no-options`
  - Pro: mirrors curl's FTP flag surface 1:1.
  - Con: each is a small patch on top of suppaftp. Ship per-request.

**SMTP / IMAP / POP3**:

- `--mail-auth <ADDR>` — SMTP original-sender address.
- `--mail-rcpt-allowfails` — allow RCPT TO failures.
- `--login-options <STR>` — IMAP/POP3 login options.
- `--sasl-authzid <ID>`, `--sasl-ir` — SASL tweaks.

**SSH**:

- `--hostpubmd5 <HEX>`, `--hostpubsha256 <SHA>` — pin SSH host key.
- `--pubkey <FILE>` — SSH public-key file.
- `--compressed-ssh` — SSH compression.

**Proxy** (detailed proxy auth + TLS — mostly curl-parity):

- `--preproxy <URL>` — chain two proxies.
- `--proxy-header <H: V>` — headers on the CONNECT request.
- `--proxy-http2`, `--proxy1.0 <HOST>`, `-p, --proxytunnel`
- `--proxy-ca-native`, `--proxy-capath`, `--proxy-crlfile`
- `--proxy-ciphers`, `--proxy-tls13-ciphers`, `--proxy-pinnedpubkey`
- `--proxy-pass`, `--proxy-tlsauthtype`, `--proxy-tlspassword`, `--proxy-tlsuser`
- `--proxy-tlsv1`

**Tracing / debug** (high value for debugging wire-level issues):

- `--trace <FILE>`, `--trace-ascii <FILE>` — hex / ASCII wire dump.
- `--trace-ids`, `--trace-time`, `--trace-config` — trace modifiers.
- `--libcurl <FILE>` — emit a C source file that reproduces the
  invocation via libcurl.
  - Pro: every advanced curl user uses `--trace-ascii` at least
    once. Recon's `-vvv` is close but not identical.
  - Con: need to hook reqwest's connector at the byte level; the
    same work that blocks `-w` phase timings. Revisit together.

**Multi-config**:

- `-K, --config <FILE>` — read flags from a file (one per line, `#`
  comments, `@FILE` for another).
- `-q, --disable` — ignore `~/.curlrc`. (recon has no analogue; if
  `--config` lands, add this alongside.)

**Variables / expansion**:

- `--variable <NAME=VAL[@FILE]>` — named variable usable as `{{NAME}}`
  in other flags.
- `--expand-*` — variant spellings that enable variable expansion
  in specific flag values.
  - Pro: lets CI configurations stay DRY without shell-side
    templating.
  - Con: substantial parser work; interacts with clap's positional
    handling. Low-value until multiple flags need it.

**Telnet**:

- `-t, --telnet-option <opt=val>` — set telnet options during a
  `telnet://` probe.

**Filesystem metadata**:

- `--xattr` — write URL / MIME type into extended attributes of
  the downloaded file. macOS / Linux.

**Misc / legacy / already-N/A**:

- `--metalink` — deprecated even in curl.
- `--egd-file` — EGD randomness source (Unix-only, legacy).
- `--manual` — curl's full manual; recon has `--examples` + `docs/MANUAL.md`.
- `--use-ascii` / `-B` — legacy FTP ASCII mode.
- `--ssl-allow-beast`, `--ssl-auto-client-cert`, `--ssl-no-revoke`,
  `--ssl-revoke-best-effort` — Windows Schannel-only (architectural
  mismatch; rustls doesn't expose these knobs).
- `--xattr` — already listed above.

**Already-present aliases worth noting**:

- recon's `-b` is `--cookiejar` (saves + loads). curl splits this into
  `-b, --cookie` (send) and `-c, --cookie-jar` (save). Behaviour
  overlaps but not 1:1 — if strict parity matters, clap aliases can
  be added.
- `--show-error` is likely worth adding as the counterpart to `-s`
  (force-show errors even when silent).

**Assessment.** Per-release phasing if curl-parity becomes a goal:

1. **Quick wins** (one release, ~500 LOC): `-F / --form` +
   `--form-string`, `-n / --netrc*`, `-z / --time-cond`,
   `-r / --range`, `--max-filesize`, `--output-dir`,
   `--oauth2-bearer`, `--remove-on-error`, `--connect-to`.
2. **HTTP version knob** (small): `--http1.0`, `--http1.1`,
   `--http2`, `--http2-prior-knowledge`. Thin reqwest wrappers.
3. **Retry layer** (medium): `--retry*` cluster. Needs a new layer
   around `client::execute`.
4. **Parallel** (medium): `-Z / --parallel`, `--parallel-max`. Needs
   an async work queue.
5. **Proxy cluster** (medium-large): `--preproxy`, `--proxy-http2`,
   `--proxy-{ca-native,capath,crlfile,ciphers,tls13-ciphers,pinnedpubkey}`.
6. **Trace/debug** (blocked): `--trace*`, `--libcurl`. Interacts with
   the same reqwest-connector issue that blocks `-w` phase timings.
7. **FTP / SMTP / IMAP / SSH specifics**: per-ask.

`-F` and `--netrc` are probably the two most-requested-in-curl-issues
omissions; worth prioritising if user demand surfaces.

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
