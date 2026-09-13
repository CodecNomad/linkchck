fn main() {
    println!("cargo:rustc-env=CONCURRENT_REQUEST_LIMIT={}", 100); // How many requests can be sent at the same time
}
