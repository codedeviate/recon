# Example Rhai scripts

These are reference scripts shipped with the recon repository. They're
not installed anywhere automatically — they sit here for you to read,
run directly, or copy into `~/.recon/script/` for bare-name invocation.

## Running directly

```sh
recon --script ./script/browser-screenshot.rhai https://example.com
```

## Copying into the global dir

```sh
recon --init                              # one-time bootstrap
cp script/*.rhai ~/.recon/script/
recon --script browser-screenshot https://example.com
```

## agentBrowser examples

These require [agent-browser](https://github.com/agent-browser/agent-browser)
on `PATH`. Every script starts with a graceful `agentBrowser::available`
guard so an un-installed `agent-browser` produces a clean exit 2 instead
of a runtime error.

| File | What it does |
|---|---|
| `browser-screenshot.rhai` | Opens `args[1]`, takes a screenshot, closes. |
| `browser-title.rhai` | Opens `args[1]` and prints the page title. |
| `browser-snapshot.rhai` | Dumps the interactive accessibility snapshot. |
| `browser-form-login.rhai` | Pattern demo — fills a two-field login form with credentials from `args[1..=3]`. |
| `browser-guard.rhai` | Shows the availability-guard pattern: prefer agent-browser but fall back to plain `http()`. |
