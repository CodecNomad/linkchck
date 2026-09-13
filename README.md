# linkchck

`linkchck` scans text/markdown files for URLs and reports links that appear to be dead.

## Usage

Run it with one or more files as arguments:

```bash
cargo run --release -- test.md > output.log
```

- `output.log` will contain links that failed checks.
- Log level is set to `info` for release versions, for debug it's set to `trace`

## Current limitations
- There is no single domain rate limiting, so if your file had the same domain with different paths 100k times, you'd spam it.

## Compile-time flags in "build.rs"
- `CONCURRENT_REQUEST_LIMIT` will allow you to change the limit of concurrent outgoing requests, default value is `100`
