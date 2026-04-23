# Example Rhai scripts

One focused `.rhai` per `recon` script-binding module. Each file
demonstrates the minimal idiom and is runnable as-is:

```sh
recon --script script/http.rhai
recon --script script/dns.rhai example.com A,MX,TXT
```

Or copy into `~/.recon/script/` for bare-name invocation:

```sh
recon --init                            # one-time bootstrap
cp script/*.rhai ~/.recon/script/
recon --script http example.com         # picks up ~/.recon/script/http.rhai
```

Scripts that need external services (Redis, memcached, MQTT, browser
automation) probe reachability first and exit 2 cleanly when the
backend isn't present — safe to run through the whole set without
surprise failures.

## Index

### Protocol probes

| Script | What it does |
|---|---|
| `http.rhai` | GET + status assertion + body length |
| `tcp.rhai` | TCP connect probe with latency |
| `ping.rhai` | TCP or ICMP ping |
| `dns.rhai` | DNS records (default bundle or custom types) |
| `tls.rhai` | Certificate inspection + days-remaining |
| `ntp.rhai` | Clock offset + round-trip delay |
| `redis.rhai` | PING or arbitrary RESP command |
| `ws.rhai` | WebSocket handshake + Ping/Pong |
| `dict.rhai` | RFC 2229 DICT lookup |
| `ldap.rhai` | Anonymous RootDSE query |
| `whois.rhai` | Two-hop whois with registrar referral |
| `memcached.rhai` | Text-protocol version + stats |
| `rtsp.rhai` | RTSP OPTIONS + method list |
| `mqtt.rhai` | MQTT publish with reachability guard |
| `smtp.rhai` | SMTP capability + STARTTLS probe |

### Sessions (scriptable `browser()`)

| Script | What it does |
|---|---|
| `browser.rhai` | Minimal stateful browser: cookies + headers stick across calls |
| `browser-login.rhai` | JSON login → protected resource with sticky session cookie |
| `browser-persist.rhai` | `use_persistent_session()` — jar survives across runs |
| `browser-multi.rhai` | Three independent browsers with different personas + jars |
| `browser-iso8859.rhai` | Browser posting to a Latin-1 service; auto-transcodes body |

### Text processing

| Script | What it does |
|---|---|
| `text.rhai` | Charset detect / decode / encode + newline normalisation |

### Data primitives

| Script | What it does |
|---|---|
| `file.rhai` | Read file as Blob + sha256 |
| `hash.rhai` | All nine hash algorithms over one payload |
| `compression.rhai` | Round-trip every stream-compression algo |
| `archive.rhai` | Create + extract a zip in /tmp |
| `sqlite.rhai` | In-memory SQLite round-trip |

### Domain tools

| Script | What it does |
|---|---|
| `encode.rhai` | QR / DataMatrix / barcode to PNG |
| `encrypt.rhai` | age keypair generation (CLI companion for full encrypt/decrypt) |
| `checkdigit.rhai` | Verify or inspect any check-digit algorithm |
| `sample.rhai` | Enumerate built-in sample sources |
| `jwt.rhai` | Sign + view + validate JWT round-trip |
| `email.rhai` | SPF / DMARC / MTA-STS / TLS-RPT / BIMI aggregate |
| `netstatus.rhai` | Connectivity probe set |

### Browser automation (external `agent-browser` CLI)

| Script | What it does |
|---|---|
| `agent-browser.rhai` | Minimal open / title / snapshot / close flow |
| `agent-browser-screenshot.rhai` | Take a screenshot |
| `agent-browser-title.rhai` | Extract the page title |
| `agent-browser-snapshot.rhai` | Accessibility-tree dump |
| `agent-browser-form-login.rhai` | Fill a two-field login form |
| `agent-browser-guard.rhai` | Prefer browser, fall back to HTTP |

Every script starts with a usage comment showing args and a one-line
description. Scripts that take positional args support a sensible
default so `recon --script NAME` alone does something useful.
