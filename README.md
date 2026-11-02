# Random Number Generator for LOKI

A portable LOKI plugin with a full generator page and a panel inside the LOKI
dashboard. Built for **LOKI 0.4.0 or newer**, using `loki-sdk` 0.4.0.

## Install the compiled plugin

1. Open **Applications** in LOKI and import `random-number-generator-0.1.0.loki-app`.
2. Grant access to the people or groups who should use it, then enable the app.
3. Choose **Open panel** to generate numbers inside LOKI, or open the app's site
   for dice presets and a copy button.

No external services, Node.js runtime, database, or extra configuration is needed.
LOKI handles sign-in and access permissions. Locally, the site is normally
`http://random-number-generator.localhost:7070` (substitute your LOKI port).

## Features

- Inclusive minimum/maximum, including negative integers and equal endpoints.
- 1–100 numbers per draw, with optional unique results.
- D6, D20, binary (0 or 1), and 1–100 presets on the full page.
- Copy results, responsive layout, and keyboard-accessible controls.
- System cryptographic randomness through WASI, with rejection sampling to avoid
  modulo bias. Unique draws use a sparse Fisher–Yates shuffle without duplicate
  retries or allocating the entire range.
- Server-side input validation and uncached JSON responses.
- No persistent history, cookies, external requests, or additional capabilities
  requested by the plugin.

Accepted endpoints are whole numbers from -9,007,199,254,740,991 through
9,007,199,254,740,991, so JSON and browsers preserve them exactly. Unique draws
cannot exceed the number of integers in the chosen range. Results in the page
last until you reload; panel results last until the panel is refreshed.

## Build from source

Install a current stable Rust toolchain, then run in this directory:

```sh
rustup target add wasm32-wasip2
cargo install cargo-loki --version 0.4.0 --locked
cargo test --locked
cargo loki package --locked
```

The package is written to
`target/loki/random-number-generator-0.1.0.loki-app`.

For a separate local development installation:

```sh
cargo loki dev --server /path/to/loki-server --port 0
```

Additional checks:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
node --check static/app.js
```

## JSON endpoint

An authenticated caller with LOKI app access can request:

```text
GET /api/generate?min=-10&max=10&count=3&unique=true
```

Example response (the actual numbers vary):

```json
{"numbers":[-7,0,8],"options":{"min":-10,"max":10,"count":3,"unique":true}}
```

Defaults: `min=1`, `max=100`, `count=1`, `unique=false`. For `unique`, `1` and
`0` are also accepted. Invalid input returns HTTP 400 and `{"error":"..."}`;
unavailable randomness returns HTTP 503. Responses use `Cache-Control: no-store`.
The endpoint creates no stored state, so generation uses GET. LOKI controls
authentication and panel action CSRF checks.

## References

- [LOKI plugin guide](https://github.com/LOKI-Server/LOKI/blob/master/docs/apps.md)
- [SDK reference](https://github.com/LOKI-Server/LOKI/blob/master/crates/loki-sdk/README.md)

MIT licensed; see LICENSE.
