use dashmap::DashSet;
use futures::future::join_all;
use regex::Regex;
use std::{
    env,
    sync::{Arc, LazyLock},
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Semaphore;
use tracing::{Level, error, info, instrument, trace, warn};

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

static CONCURRENT_REQUEST_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(env!("CONCURRENT_REQUEST_LIMIT").parse().unwrap()));

static ALREADY_CHECKED: LazyLock<Arc<DashSet<String>>> =
    LazyLock::new(|| Arc::new(DashSet::<String>::new()));

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"https?:\/\/(www\.)?[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_+.~#?&/=]*)",
    )
    .expect("This shall not be a problem")
});

#[instrument(level = "trace")]
async fn check_url(url: String) {
    trace!("Checking url");
    if !ALREADY_CHECKED.insert(url.clone()) {
        info!("Skipping duplicate url, already being checked: {}", url);
        return;
    };
    info!("Checking url: {}", url);

    let _permit = CONCURRENT_REQUEST_SEMAPHORE.acquire().await.unwrap();
    if REQWEST_CLIENT.head(&url).send().await.is_err() {
        warn!("Found dead link: {}", url);
        println!("{}", url);
    };
}

#[instrument(level = "trace")]
async fn match_line(line: String) {
    trace!("Matching urls");
    let task_futures = REGEX.find_iter(&line).map(|url_match| {
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
