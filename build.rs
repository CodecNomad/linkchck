use tracing::Level;

fn main() {
    let concurrent_request_limit = 100;
    let max_trace_level = Level::INFO;

    println!(
        "cargo:rustc-env=CONCURRENT_REQUEST_LIMIT={}",
        concurrent_request_limit
    );
    println!("cargo:rustc-env=TRACE_LEVEL={}", max_trace_level);
}
