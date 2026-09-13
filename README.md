# linkchck

`linkchck` scans text files for URLs and prints links that fail reachability checks.

## Usage

Run it with one or more files as positional arguments:

```bash
cargo run --release -- /path/to/file.md /path/to/another.txt > dead-links.txt
```

- Failed URLs are printed to `stdout` (so they can be redirected to a file).
- Logs are written to `stderr`.
- Log level is `trace` in debug builds and `info` in release builds.

## How it works

- Extracts URLs from each input line with `linkify`.
- Tries `HEAD` first, then falls back to `GET` when needed.
- Treats non-success responses and request failures as dead links.
- Deduplicates URLs within a run so each URL is checked once.
- Limits concurrent outbound requests with a semaphore.

## Project layout

```text
.
├── src/
│   └── main.rs   # CLI entrypoint and link-checking logic
├── build.rs      # sets compile-time CONCURRENT_REQUEST_LIMIT
├── test.md       # sample input file
└── Cargo.toml    # package metadata and dependencies
```

## Compile-time configuration

`build.rs` sets `CONCURRENT_REQUEST_LIMIT` to `100` by default, which is used to cap concurrent HTTP requests.
