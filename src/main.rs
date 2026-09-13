use dashmap::DashSet;
use futures::future::join_all;
use regex::Regex;
use std::{
    env, fs,
    io::{BufRead, BufReader},
    sync::{Arc, LazyLock},
};
use tokio::sync::Semaphore;
use tracing::{Level, error, info, instrument, trace, warn};

static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

static CONCURRENT_REQUEST_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(env!("CONCURRENT_REQUEST_LIMIT").parse().unwrap()));

static ALREADY_CHECKED: LazyLock<Arc<DashSet<String>>> =
    LazyLock::new(|| Arc::new(DashSet::<String>::new()));

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:http[s]?:\/\/.)?(?:www\.)?[-a-zA-Z0-9@%._\+~#=]{2,256}\.[a-z]{2,6}\b(?:[-a-zA-Z0-9@:%_\+.~#?&\/\/=]*)",
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
async fn check_line(result_line: std::result::Result<std::string::String, std::io::Error>) {
    trace!("Checking line");
    let line = match result_line {
        Ok(line) => line,
        Err(err) => {
            error!(?err, "Unable to read line");
            return;
        }
    };

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
    let file_handle = match fs::File::open(&file_path) {
        Ok(handle) => handle,
        Err(err) => {
            error!(?err, "Unable to open handle");
            return;
        }
    };

    trace!("Reading lines");
    let buf_reader = BufReader::new(file_handle);
    let future_tasks = buf_reader
        .lines()
        .map(|line| tokio::spawn(check_line(line)));

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
