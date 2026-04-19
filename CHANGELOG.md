# Changelog

All notable changes to recon are recorded here. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For pre-0.4.1 design context and architectural notes, see [HISTORY.md](HISTORY.md).

## [Unreleased]

## [0.17.0] - 2026-04-19

### Added

- Complete EU-27 VAT check-digit coverage — 22 new country algorithms:
  - Primary keywords (auto-detect where the country has sub-variants):
    `at-vat`, `be-vat`, `bg-vat`, `cy-vat`, `cz-vat`, `ee-vat`, `el-vat`
    (alias `gr-vat`), `es-vat`, `hr-vat`, `hu-vat`, `ie-vat`, `it-vat`,
    `lt-vat`, `lu-vat`, `lv-vat`, `mt-vat`, `nl-vat`, `pl-vat`, `pt-vat`,
    `ro-vat`, `si-vat`, `sk-vat`.
  - Explicit sub-variant keywords for multi-variant countries:
    - Spain: `es-nif` (citizen), `es-nie` (foreigner), `es-cif` (entity).
    - Bulgaria: `bg-egn` (personal, date-validated), `bg-bulstat` (legal).
    - Czech Republic: `cz-person`, `cz-legal`.
    - Latvia: `lv-person`, `lv-business`.
- Each country uses its native algorithm — weighted mod-11, weighted
  mod-10, mod-89, mod-97, mod-26 letter lookup, ISO 7064 MOD 11-10,
  or Luhn. Auto-detect surfaces the matched variant in the verify
  output's type field.
- Help topic updated with all 27 EU VAT keywords and sub-variants.
- ~116 new unit tests covering known VAT vectors per country.

### Changed

- `src/checkdigit/vat.rs` (a single file in 0.16.0) is now a directory
  `src/checkdigit/vat/` with one file per country (27 total). Each
  country's algorithm and tests live in a focused file. The five 0.16.0
  countries migrated with no logic change.

### Reserved for 0.18.0

- Non-EU European VAT / company-ID jurisdictions — NO, UK, CH, IS, LI,
  RS, UA, TR, RU, BY, MD, MK, ME, AL, BA, XK — will land as a follow-up
  release.

## [0.16.0] - 2026-04-19

### Added

- `--checkdigit <NAME>` / `--checkdigit-create <NAME>` — verify or compute
  check digits across 40 canonical algorithms (55 total keywords with aliases):
  - Luhn family: `luhn`, `creditcard` (auto-detect), `visa`, `mastercard`
    (alias `mc`), `amex`, `discover`, `jcb`, `imei`, `isin`, `npi`,
    `personnummer` (`se-id`), `sin` (`ca-sin`), `sa-id`.
  - EAN / GTIN family (mod 10 alternating 1×/3×): `ean13` (`ean`),
    `ean8`, `upca` (`upc`), `upce`, `isbn13`, `gtin8`, `gtin12`,
    `gtin13`, `gtin14` (`gtin`), `sscc`.
  - `isbn10` — mod 11, allows `X` as check digit.
  - Personal IDs via mod 11: `cpr` (`dk-id`), `bsn` (`nl-id`),
    `fodselsnummer` (`no-id`, two check digits). Post-2007 Danish CPRs
    may legitimately fail the check — noted in the help topic.
  - `henkilotunnus` (`fi-id`) — Finnish mod-31 with extended 2023 century
    markers (`A`-`F` for 2000s).
  - `iban` — mod 97 with 80+ country length table.
  - `vin` — transliterate + weighted mod 11; check at position 9; `I`,
    `O`, `Q` disallowed.
  - `mrz` — ICAO Doc 9303 passport / ID MRZ (TD1/TD2/TD3 formats).
  - `aba` (`us-routing`) — US bank routing number.
  - Cryptocurrency: `btc` (`bitcoin`), `ltc` (`litecoin`), `doge`
    (`dogecoin`) — base58check; `eth` (`ethereum`, `eip55`) — EIP-55
    mixed-case; `bech32` (`segwit`) — BIP-173.
  - EU VAT starter (5 countries): `se-vat`, `dk-vat`, `fi-vat`, `de-vat`,
    `fr-vat`. Remaining 22 EU countries + non-EU European jurisdictions
    reserved for 0.17.0.
- `--checkdigit-list` — standalone action: prints the full algorithm /
  alias table.
- `--raw` — strips grouping from output (applies to `--checkdigit` and
  `--checkdigit-create` only).
- New `recon --help checkdigit` help topic with complete documentation.
- New `CHECK DIGITS` section in `recon --examples`.
- New dependencies: `bs58 = "0.5"`, `bech32 = "0.11"`. Existing `sha2`
  and `sha3` reused for Bitcoin SHA-256d and Ethereum Keccak-256.

### Changed

- New `src/checkdigit/` module directory (17 files) following the same
  pattern as `src/email/` and `src/serve/`. 121 new unit tests.

### Internal

- `source::read_all` is now used by `--checkdigit` (matching `--hash` /
  `--encrypt`'s source-layer integration).

## [0.15.2] - 2026-04-19

### Changed

- `--help` output is now colorized (when stdout is a TTY): section headings
  (`HTTP Request:`, `Auth & TLS:`, etc.) and the `Usage:` line render in
  yellow + bold, flag names (`-H, --header`) in cyan, and argument
  placeholders (`<URL>`, `<PATH>`) in green. Matches the existing
  `--examples` palette. ANSI codes are suppressed when output is piped.

## [0.15.1] - 2026-04-19

### Changed

- `--help` output is now grouped into labeled sections (`HTTP Request`,
  `Auth & TLS`, `Output`, `Certificate Inspection`, `DNS`, `WHOIS`,
  `Network Tests`, `Email Protection`, `Cookies`, `File Server`, `JWT`,
  `Hashing`, `Compression`, `Encoding`, `Encryption`, `Sample Data`,
  `Editor`, `Meta`) matching the existing `--help <topic>` taxonomy.
  No flag names changed; only the rendering of `--help`.

### Added

- `CHANGELOG.md` at the repo root, covering versions 0.15.0 → 0.7.0
  (lifted from `HISTORY.md`), plus reconstructed entries for 0.6.0
  (JWT), 0.5.0 (SSH/Telnet), 0.4.1 (DMARC cross-validation fix), and
  0.1.0 (initial curl-clone baseline). Versions 0.2.0–0.4.0 are
  documented chronologically in `HISTORY.md`'s Feature Additions
  narrative.

### Removed

- The `## Version Log` section of `HISTORY.md`. Its content lives in
  `CHANGELOG.md` now. `HISTORY.md` retains the design-notes narrative
  (Origins, Architecture Decisions, Feature Additions, Naming History,
  Module Structure, Dependencies).

## [0.15.0] - 2026-04-19

### Added

- `--encrypt` / `--decrypt`: age-format encryption and decryption over any
  source (file, URL, stdin, file://). Output to stdout or `-o <FILE>`.
  Binary by default; `--armor` produces ASCII-armored output for paste
  into email or chat.
- `--passphrase-file <PATH>`, `$RECON_PASSPHRASE` env var, and an
  interactive hidden prompt as the three passphrase sources (priority in
  that order). The `--passphrase <TEXT>` literal flag is intentionally not
  offered (security footgun).
- `--recipient <AGE1... | PATH>` (repeatable): encrypt to one or more
  X25519 recipients. Mix with a passphrase in the same invocation; any
  one recipient or the passphrase decrypts.
- `--identity <PATH>` (repeatable): decrypt using an age private-key
  file. Skips blank and `#`-comment lines.
- `--encrypt-keygen`: standalone action that generates a fresh X25519
  key pair (age-compatible). Prints the public key as a comment and the
  private key on its own line.

## [0.14.1] - 2026-04-18

### Fixed

- `--encode code128`: no longer requires users to manually prepend a
  code-set marker (À/Ɓ/Ć). Code-set B (mixed ASCII) is now automatically
  selected when the input does not already start with one.
- `--encode ean13`: now accepts 12-digit input (body without check) in
  addition to the full 13-digit code; the check digit is computed automatically.
- `--encode upca`: now accepts 11-digit input (body without check) in
  addition to the full 12-digit code; the check digit is computed automatically.

## [0.14.0] - 2026-04-18

### Added

- `--encode <FORMAT>`: generate a QR, DataMatrix, Code 128, Code 39,
  EAN-13, or UPC-A code from the positional text. Use `-` or a pipe to
  read from stdin; `--from-file <PATH>` reads from disk.
- `--encode-format <ascii|svg|png>`: output format. When omitted, inferred
  from `-o <FILE>` extension (`.svg` / `.png`); defaults to ASCII otherwise.
- `--from-file <PATH>`: read encode input from a file instead of the
  positional argument. Mutually exclusive with a positional text value.
- `--encode-list`: standalone action listing the supported formats.

## [0.13.0]

### Added

- `--compress <ALGO>`: compress the input source (file, URL, stdin, file://)
  with gzip, deflate, zstd, brotli, or bzip2. Streams bytes; output goes to
  stdout or -o <FILE>.
- `--decompress [ALGO]`: decompress the input source. Without ALGO, the
  first 6 bytes are inspected and gzip / zstd / bzip2 are auto-detected.
  Deflate and brotli lack magic bytes; pass the algorithm explicitly for
  those.
- `--compression-level <LEVEL>`: quality for --compress. Accepts a number
  in the algorithm's native range or a word (fastest, fast, default, good,
  best). Invalid with --decompress.
- `--compress-list`: standalone action listing supported algorithms with
  their aliases, magic bytes, and level ranges.

## [0.12.0]

### Added

- `--hash <ALGO>`: compute a cryptographic hash of any source — local file,
  `file://` URL, HTTP(S) URL, or stdin. Supported: md5, sha1, sha256,
  sha384, sha512, sha3-256, sha3-512, blake3 (case-insensitive; hyphens
  and underscores accepted). HTTP sources honour every usual HTTP flag
  (`-H`, `-u`, `-L`, `-k`, `-A`, cookies, `-e`).
- `--hash-format <hex|base64|raw>`: digest output format. Default is
  lowercase hex. `raw` writes binary bytes with no trailing newline.
- `--hash-list`: standalone action that lists supported algorithms and
  their digest sizes. Does not require a URL.

### Changed

- Internal: new `src/source.rs` module unifies input-source handling
  (file, URL, stdin, `file://`) for feature flags that consume arbitrary
  bytes. No user-visible change on its own; backs `--hash` and the
  upcoming `--compress` / `--encrypt` / `--qr` / `--barcode` flags.

## [0.11.0]

### Added

- `-e, --referer <URL>`: send a Referer header (alias `--referrer` for the
  common misspelling). Overridden by any explicit `-H "Referer: …"`.
- `-O, --remote-name`: save the response body to a file named after the
  URL's final path segment (mirrors `curl -O`). Percent-decodes the name;
  errors when the URL has no filename or when the name would escape the
  current directory. Mutually exclusive with `-o/--output`.
- `-T, --upload-file <PATH>`: upload a local file as the request body.
  Defaults the method to PUT unless `-X` is set explicitly. Mutually
  exclusive with `-d/--data`.

### Changed

- `-X/--request` is now `Option<String>`; the effective method is resolved
  through a new `Args::effective_method()` helper that honours explicit
  `-X`, then `-T` (PUT), then `-d`/`-G` (POST/GET), then defaults to GET.
  No user-visible behaviour change for existing invocations.

## [0.10.1]

### Changed

- `--sample lorem` output now always begins with the words "Lorem ipsum" —
  the first paragraph's opener is fixed, the first two words in word mode
  are fixed, and character mode starts with the prefix (truncated if
  `--sample-count` is smaller than 11). Remaining content stays random.

## [0.10.0]

### Changed

- `--sample lorem` now produces randomized output instead of deterministic
  rotation through the corpus. Each invocation yields different text.

### Added

- `--sample-seed <N>`: optional u64 seed for lorem generation. When set,
  output is reproducible for that seed + count + unit. When omitted, a
  seed is derived from the current system time.
- Using `--sample-seed` with a non-lorem sample is a loud error.

## [0.9.0]

### Added

- `--sample <NAME[:FORMAT[:COUNT]]>`: fetch sample ecommerce data from known
  free APIs. Built-in names: `customer`, `product`, `order`, `category`,
  `address`, `image`, and a local `lorem` generator. Composes with `-p`,
  `-o`, `--editor`, `-i`, and verbose.
- `--sample-format <FMT>` and `--sample-count <N[p|w|c]>`: override format
  and count independently of the colon shortcut. Unit suffixes (`p`, `w`,
  `c`) are only valid for the local `lorem` sample; passing them to other
  samples is an error.
- `--sample-file [PATH]`: write sample output to file(s). Default filename
  is `sample-{{name}}.{{format}}` (bulk) or `sample-{{name}}-{{n}}.{{format}}`
  (per_item). Required when `per_item` sample is fetched with count > 1.
- `--sample-list`: standalone action listing all available samples (built-in
  plus user-configured). Does not require a URL.
- `[sampledata.<name>]` config sections in `~/.recon/config.toml`: add or
  override samples with `mode`, `default_format`, `count`, `urls.<fmt>`,
  `headers`, `basic_auth`, and `description`. `${ENV_VAR}` substitution is
  honored in URL, header, and basic_auth strings so secrets stay out of the
  config file.

## [0.8.0]

### Added

- `--editor [EDITOR]`: redirect response output to a file and open it in an editor.
  Accepts a built-in alias (`zed`, `code`, `cursor`, `subl`, `vim`, `nvim`, `nano`,
  `emacs`), a user-defined alias from `~/.recon/config.toml`, or a raw shell
  command. When the flag is given with no value, falls back to
  `[editor] default` in the config. The temp file is written to
  `/tmp/recon-<unix-ms>.<ext>` with the extension derived from the response
  `Content-Type`. By default stdout is silent; `-vv` or higher also mirrors the
  body to stdout.
- `--editor-cleanup`: remove all `/tmp/recon-*` temp files from previous
  `--editor` invocations. Standalone action — does not require a URL.
- `[editor]` config section in `~/.recon/config.toml`:
  `default` (optional alias used when `--editor` has no value) and
  `[editor.aliases]` (optional map of user-defined alias → command).

## [0.7.3]

### Changed

- SNI directory parsing now accepts both `<hostname>-cert.pem` and `<hostname>.pem` as certificate files when paired with `<hostname>-key.pem`, improving compatibility with mkcert-style output.

## [0.7.2]

### Changed

- `--serve-sni` now accepts an omitted value and defaults to `~/.recon/sni/`, so `recon --serve-sni` uses that directory automatically.

## [0.7.0]

### Added

- `--netstatus`: connectivity checker that runs configurable probes concurrently
  and exits non-zero if any check fails (suitable for scripting with `--silent`).
- `~/.recon/config.toml`: new general config file; `[netstatus]` is the first section.
- Supported probe schemes: `http://`, `https://`, `ping://`, `tcp://`, `tls://`,
  `dns://`, `ntp://`.
- Public IP cross-check: fetches from multiple sources and flags disagreement.
- `[[netstatus.dns_hijack_checks]]`: per-server DNS hijack detection with expected-IP assertion.

## [0.6.0]

### Added

- `--jwt-view`, `--jwt-sign`, `--jwt-validate`: sign, validate, and inspect JWT
  tokens without leaving the terminal. `--jwt-view` decodes and pretty-prints
  the header and payload without signature verification. `--jwt-sign` signs a
  JWT from a JSON payload, partial token (header.payload), or bare base64
  payload; `iat` is added automatically if missing. `--jwt-validate` verifies
  the HMAC signature and, with opt-in flags, checks individual claims.
- Claim flags for signing and validation: `--jwt-iss`, `--jwt-sub`, `--jwt-aud`,
  `--jwt-exp`, `--jwt-nbf`, `--jwt-iat`, `--jwt-jti`.
- Validation toggles: `--jwt-validate-exp`, `--jwt-validate-nbf`,
  `--jwt-validate-iat`, `--jwt-validate-iss`, `--jwt-validate-sub`,
  `--jwt-validate-aud`, `--jwt-validate-jti`, `--jwt-validate-full`.
- Supporting flags: `--jwt-secret` (HMAC secret), `--jwt-alg` (HS256, HS384,
  HS512; default HS256), `--jwt-json-report` (machine-readable output for
  `--jwt-view` and `--jwt-validate`).
- Input for JWT operations can come from `-d <string>`, `-d @file`, a
  positional file path, or stdin.

## [0.5.0]

### Added

- `ssh://[user@]host[:port]`: interactive SSH PTY shell on the remote server.
  Reuses the existing SCP auth stack (agent → key → password, host key
  verification via `~/.ssh/known_hosts`). Terminal resize is forwarded via
  SSH `window-change` requests. Shared auth helpers extracted into
  `src/ssh_auth.rs`.
- `telnet://host[:port]`: Telnet client with full IAC option negotiation per
  RFC 854. Accepts `WILL ECHO` and `WILL SUPPRESS-GO-AHEAD` from the server;
  rejects all others with DONT/WONT. Subnegotiation blocks are discarded.
  `0xFF` bytes in input are escaped as `IAC IAC`.

## [0.4.1]

### Fixed

- `--dmarc` run on its own no longer emits spurious `[⚠ WARN]` cross-validation
  notes suggesting the user add `--spf` or `--dkim`. Those notes were
  suggestions, not findings, and cluttered output when only DMARC was
  requested. The DMARC+SPF and DMARC+DKIM "not checked" entries were removed
  from `cross_validate()`.

## [0.1.0]

### Added

- Initial curl-clone baseline: HTTP/HTTPS requests (`GET`, `POST`, `PUT`,
  `DELETE`, `PATCH`, `HEAD`), custom headers (`-H`), request body (`-d`,
  `@file`), redirect following (`-L`, `--max-redirs`), file output (`-o`)
  with progress bar, silent (`-s`) and verbose (`-v`) modes, response
  header inclusion (`-i`), custom User-Agent (`-A`), connection timeout
  (`--connect-timeout`), fail-on-HTTP-error (`-f`).

---

*Versions 0.2.0 through 0.4.0 were developed before release tags were recorded.
Intermediate features (TLS certificate inspection, DNS, WHOIS, ping, traceroute,
redirect header tracing, prettification, status-only output, cookies, SCP,
email protection checks, per-topic help, file server, SNI, output model
overhaul) landed across that range and are documented chronologically in
[HISTORY.md](HISTORY.md) §4–§19.*
