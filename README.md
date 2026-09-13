# linkchck

`linkchck` scans text/markdown files for URLs and reports links that appear to be dead.

## Usage

Run it with one or more files as arguments:

```bash
cargo run -- test.md > output.log 2> tracing.log
```

- `output.log` will contain links that failed checks.
- `tracing.log` will contain tracing/log output.

## Current limitations

- There is no rate limiting yet.
- Do not use it on files with many links to the same domain, or it may send too many requests in too short a time.
- It may also fail when too many different domains are requested at the same time.

## TODO

- [ ] Add request rate limiting (especially per domain).
- [ ] Add concurrency controls/backpressure for large multi-domain workloads.
