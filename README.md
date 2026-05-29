# crstop

`crstop` is designed for checking usage from a terminal when [claude-relay-service](https://github.com/Wei-Shaw/claude-relay-service) is only reachable through an SSH-accessible intranet relay. If you are already inside the intranet and can open the CRS web UI, the browser dashboard is still the simplest option.

## What It Does

- Shows a btop/nvitop-style terminal dashboard for the CRS instance configured by Codex.
- Reads `~/.codex/config.toml`, derives the CRS root URL, and resolves `preferred_auth_method = "apikey"` through `~/.codex/auth.json` (`OPENAI_API_KEY`).
- Masks the API key in output and only caches the derived `apiId` under `~/.cache/crstop/api-id.json`.
- Checks CRS health, Redis status, current key state, total/daily/monthly usage, and key-level limits.
- Does not send real model requests, so it does not consume model tokens.

## Usage

```bash
crstop                # full-screen TUI, refresh every 1 second
crstop --refresh 2    # refresh every 2 seconds
crstop --refresh 0.5  # faster half-second refresh
crstop --once       # one terminal snapshot, no fullscreen
crstop --config PATH
crstop --no-cache
```

Keys: `q` / `Esc` / `Ctrl-C` quit, `r` refreshes now, `d` and `m` toggle table detail.

## Limits

CRS Codex/OpenAI pool limits require CRS admin/account permissions. Without `CRS_ADMIN_TOKEN`, `crstop` reports `Pool Limits: not visible without admin token` instead of trying to bypass permissions.

## Build

```bash
cargo test
cargo build --release
install -m 700 target/release/crstop ~/.local/bin/crstop
```
