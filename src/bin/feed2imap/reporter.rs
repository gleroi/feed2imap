use anyhow::Error;
use feed2imap::sync::{self, Reporter};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct CliReporter {
    mb: MultiProgress,
    style: ProgressStyle,
    index: Arc<Mutex<HashMap<String, ProgressBar>>>,
}

impl CliReporter {
    pub fn new() -> Result<CliReporter, Error> {
        Ok(CliReporter {
            mb: MultiProgress::new(),
            style: ProgressStyle::default_bar()
                .progress_chars("##-")
                .template("[{prefix:20}] {wide_bar} {msg} [{pos:>4} / {len:4}]")?,
            index: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    async fn add(&self, url: &str) {
        let pb = ProgressBar::new(0);
        pb.set_style(self.style.clone());
        let pb2 = self.mb.add(pb);
        let mut index = self.index.lock().await;
        index.insert(url.to_string(), pb2);
    }
}

impl sync::Reporter for CliReporter {
    async fn on_begin(&self, feed: &str) {
        self.add(feed).await;
    }

    async fn on_entries_count(&self, feed: &str, title: &str, count: u64) {
        let index = self.index.lock().await;
        if let Some(pb) = index.get(feed) {
            pb.set_prefix(title.to_owned());
            pb.set_length(count);
        }
    }

    async fn on_entry(&self, feed: &str) {
        let index = self.index.lock().await;
        if let Some(pb) = index.get(feed) {
            pb.inc(1)
        }
    }

    async fn on_end(&self, feed: &str, result: &Result<(), Error>) {
        let index = self.index.lock().await;
        if let Some(pb) = index.get(feed) {
            if let Err(err) = result {
                pb.set_message(format!("{}", err));
            }
        }
    }
}

#[derive(Clone)]
pub struct SimpleReporter {}

impl Reporter for SimpleReporter {
    async fn on_begin(&self, feed: &str) {
        println!("tfetching: {}", feed);
    }

    async fn on_entries_count(&self, feed: &str, title: &str, count: u64) {
        println!("fetched: {} ({}) has {} entries", feed, title, count);
    }

    async fn on_entry(&self, feed: &str) {
        println!("processed: {} one more !", feed);
    }

    async fn on_end(&self, feed: &str, result: &Result<(), Error>) {
        if let Err(err) = result {
            println!("ERROR: {}: {}", feed, err);
        } else {
            println!("synced: {}", feed);
        }
    }
}
