#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use dashmap::DashSet;
use linkify::LinkFinder;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;
use std::{env, sync::LazyLock};
use tokio::fs;
use tokio::sync::Semaphore;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    task::JoinSet,
};
use tracing::{Level, error, info, warn};

const SPOOFED_HEADERS: [(&str, &str); 11] = [
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
    ),
    (
        "accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
    ),
    ("accept-language", "en-US,en;q=0.9"),
    (
        "sec-ch-ua",
        "\"Chromium\";v=\"128\", \"Not;A=Brand\";v=\"24\", \"Google Chrome\";v=\"128\"",
    ),
    ("sec-ch-ua-mobile", "?0"),
    ("sec-ch-ua-platform", "\"Windows\""),
    ("sec-fetch-dest", "document"),
    ("sec-fetch-mode", "navigate"),
    ("sec-fetch-site", "none"),
    ("sec-fetch-user", "?1"),
    ("upgrade-insecure-requests", "1"),
];

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::with_capacity(SPOOFED_HEADERS.len());
    for (name, value) in SPOOFED_HEADERS {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .expect("Failed to build reqwest client")
});

static CONCURRENT_REQUEST_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(env!("CONCURRENT_REQUEST_LIMIT").parse().unwrap()));

static ALREADY_CHECKED: LazyLock<DashSet<String>> = LazyLock::new(DashSet::<String>::new);

static LINK_FINDER: LazyLock<LinkFinder> = LazyLock::new(LinkFinder::new);

async fn check_url(url: String) {
    if !ALREADY_CHECKED.insert(url.clone()) {
        info!("Skipping duplicate url, already being checked: {}", url);
        return;
    };
    info!("Checking url: {}", url);

    let permit = CONCURRENT_REQUEST_SEMAPHORE.acquire().await.unwrap();
    let head_request = REQWEST_CLIENT.head(&url).send().await;

    let dead_link = || {
        drop(permit);
        warn!("Found dead link: {}", url);
        println!("{}", url)
    };

    if let Ok(res) = head_request {
        if res.status().is_success() {
            return;
        }

        if REQWEST_CLIENT
            .get(&url)
            .send()
            .await
            .is_ok_and(|resp| resp.status().is_success())
        {
            return;
        };
    }

    dead_link();
}

async fn read_file(file_path: String) {
    info!("Reading from file: {}", file_path);

    let file_handle = match fs::File::open(&file_path).await {
        Ok(handle) => handle,
        Err(err) => {
            error!(?err, "Unable to open handle");
            return;
        }
    };

    let mut lines = BufReader::new(file_handle).lines();
    let mut future_tasks = JoinSet::new();
    while let Ok(Some(line)) = lines.next_line().await {
        for url_match in LINK_FINDER.links(&line) {
            let owned_url = url_match.as_str().to_string();

            future_tasks.spawn(check_url(owned_url));
        }
    }

    future_tasks.join_all().await;
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(env!("TRACE_LEVEL")).unwrap())
        .init();

    let file_paths = env::args().skip(1);
    if file_paths.len() < 1 {
        error!("No file paths were given in arguments");
        return info!("Usage: linkchck arg1 arg2 ...");
    }

    let mut future_tasks = JoinSet::new();
    file_paths.for_each(|file| {
        future_tasks.spawn(read_file(file));
    });

    future_tasks.join_all().await;
}
