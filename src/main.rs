use dashmap::DashSet;
use futures::future::join_all;
use linkify::LinkFinder;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT};
use std::{
    env,
    sync::{Arc, LazyLock},
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Semaphore;
use tracing::{Level, error, info, instrument, trace, warn};

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();

    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        ),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    headers.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            "\"Chromium\";v=\"128\", \"Not;A=Brand\";v=\"24\", \"Google Chrome\";v=\"128\"",
        ),
    );
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert(
        "sec-ch-ua-platform",
        HeaderValue::from_static("\"Windows\""),
    );
    headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
    headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
    headers.insert("sec-fetch-site", HeaderValue::from_static("none"));
    headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
    headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect("Failed to build reqwest client")
});

static CONCURRENT_REQUEST_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(env!("CONCURRENT_REQUEST_LIMIT").parse().unwrap()));

static ALREADY_CHECKED: LazyLock<Arc<DashSet<String>>> =
    LazyLock::new(|| Arc::new(DashSet::<String>::new()));

static LINK_FINDER: LazyLock<LinkFinder> = LazyLock::new(LinkFinder::new);

#[instrument(level = "trace")]
async fn check_url(url: String) {
    trace!("Checking url");
    if !ALREADY_CHECKED.insert(url.clone()) {
        info!("Skipping duplicate url, already being checked: {}", url);
        return;
    };
    info!("Checking url: {}", url);

    let _permit = CONCURRENT_REQUEST_SEMAPHORE.acquire().await.unwrap();
    let head_request = REQWEST_CLIENT.head(&url).send().await;

    let dead_link = || {
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

#[instrument(level = "trace")]
async fn match_line(line: String) {
    trace!("Matching urls");
    let task_futures = LINK_FINDER.links(&line).map(|url_match| {
        let owned_url = url_match.as_str().to_string();

        tokio::spawn(check_url(owned_url))
    });

    join_all(task_futures).await;
}

#[instrument(level = "trace")]
async fn read_file(file_path: String) {
    info!("Reading from file: {}", file_path);

    trace!("Opening file_handle");
    let file_handle = match tokio::fs::File::open(&file_path).await {
        Ok(handle) => handle,
        Err(err) => {
            error!(?err, "Unable to open handle");
            return;
        }
    };

    trace!("Reading lines");
    let mut lines = BufReader::new(file_handle).lines();
    let mut future_tasks = Vec::new();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                future_tasks.push(tokio::spawn(match_line(line)));
            }
            Ok(None) => {
                break;
            }
            Err(err) => {
                error!(?err, "Unable to read line");
                break;
            }
        }
    }

    join_all(future_tasks).await;
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            Level::TRACE
        } else {
            Level::INFO
        })
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(false)
        .init();

    let future_tasks = env::args()
        .skip(1)
        .map(|file| tokio::spawn(read_file(file)));

    join_all(future_tasks).await;
}
