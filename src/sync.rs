use std::{marker::Send, sync::Arc};

use anyhow::Error;
use futures::future::try_join_all;

use crate::{fetch, transform};

pub struct Syncer {
    name: String,
    email: String,
}

pub trait Output {
    fn contains(&self, id: &str) -> bool;
    fn append(
        &self,
        mail: &Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), Error>> + std::marker::Send;
}

pub trait Reporter {
    fn on_feed(&self, feed: &str) -> impl std::future::Future<Output = ()> + std::marker::Send;
    fn on_entries_count(
        &self,
        feed: &str,
        title: &str,
        count: u64,
    ) -> impl std::future::Future<Output = ()> + std::marker::Send;
    fn on_entry(&self, feed: &str) -> impl std::future::Future<Output = ()> + Send;
}

impl Syncer {
    pub fn new(name: &String, email: &String) -> Arc<Syncer> {
        Arc::new(Syncer {
            name: name.clone(),
            email: email.clone(),
        })
    }

    pub async fn sync<TOutput, TReporter>(
        self: Arc<Self>,
        inputs: &Vec<String>,
        output: TOutput,
        reporter: TReporter,
    ) -> Result<(), Error>
    where
        TOutput: Output + Sync + Send + Clone + 'static,
        TReporter: Reporter + Send + Clone + 'static,
    {
        let mut tasks = Vec::with_capacity(inputs.len());
        for input in inputs {
            let task_input = input.clone();
            let task_output = output.clone();
            let task_self = self.clone();
            let task_reporter = reporter.clone();
            let task = tokio::spawn(async {
                task_self
                    .sync_feed(task_input, task_output, task_reporter)
                    .await?;
                Ok::<(), Error>(())
            });
            tasks.push(task);
        }
        let _results = try_join_all(tasks).await?;
        Ok(())
    }

    async fn sync_feed<TOutput, TReporter>(
        self: Arc<Self>,
        url: String,
        output: TOutput,
        reporter: TReporter,
    ) -> Result<(), Error>
    where
        TOutput: Output,
        TReporter: Reporter,
    {
        log::info!("syncing {}", url);
        reporter.on_feed(&url).await;
        let full_feed = fetch::url(&url).await?;
        let title: String = transform::extract_feed_title(&full_feed)?
            .chars()
            .take(20)
            .collect();
        reporter
            .on_entries_count(&url, &title, full_feed.entries.len() as u64)
            .await;
        for entry in &full_feed.entries {
            let id = transform::extract_message_id(&full_feed, &entry);
            if !output.contains(&id) {
                let mail = transform::extract_message(&self.name, &self.email, &full_feed, entry)?;
                log::debug!("{}: {} appending to mail", url, id);
                output.append(&mail).await?;
                log::debug!("{}: {} appended to mail", url, id);
            } else {
                log::debug!("{}: {} already in mail", url, id);
            }
            reporter.on_entry(&url).await;
        }
        Ok(())
    }
}
