use std::{
    collections::{HashMap},
    sync::Arc,
};

use anyhow::{anyhow, Error};
use clap::{Args, Parser, Subcommand};
use directories::BaseDirs;
use feed2imap::{fetch, imap, sync};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::sync::Mutex;

pub mod config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// path to configuration file, default to ~/.config/feed2imap.toml
    #[arg(long, env = "FEED2IMAP_CONFIG")]
    config: Option<String>,

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    fn config_path(&self) -> String {
        if let Some(ref config_path) = self.config {
            return config_path.clone();
        } else {
            let dirs = BaseDirs::new().unwrap();
            let config_dir = dirs.config_dir();
            let config_path = config_dir.join("feed2imap.toml");
            return config_path.to_string_lossy().into_owned();
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// add a feed
    #[command()]
    Add(AddArgs),

    /// print a default configuration
    #[command()]
    Config,

    /// list feeds
    #[command()]
    List,

    /// fetch feeds and send new entries by mail
    #[command()]
    Sync,
}

#[derive(Args)]
struct AddArgs {
    /// url of the feed
    url: String,
}

#[tokio::main]
async fn main() -> () {
    pretty_env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Command::Add(ref args) => add_feed(&cli, args).await.unwrap(),
        Command::Config => config(&cli).await.unwrap(),
        Command::List => list_feeds(&cli).await.unwrap(),
        Command::Sync => sync_feeds(&cli).await.unwrap(),
    }
}

async fn config(_cli: &Cli) -> Result<(), Error> {
    config::dump_default()
}

#[derive(Clone)]
struct CliReporter {
    mb: MultiProgress,
    style: ProgressStyle,
    index: Arc<Mutex<HashMap<String, ProgressBar>>>,
}

impl CliReporter {
    fn new() -> Result<CliReporter, Error> {
        Ok(CliReporter {
            mb: MultiProgress::new(),
            style: ProgressStyle::default_bar()
                .progress_chars("##-")
                .template("[{prefix:20}] {wide_bar} [{pos:>4} / {len:4}]")?,
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
    async fn on_feed(&self, feed: &str) {
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
}

async fn sync_feeds(cli: &Cli) -> Result<(), Error> {
    let config = Arc::new(config::load(&cli.config_path())?);
    log::debug!("connecting to mail server");
    let client = imap::client(&config.imap.username, &config.imap.password).await?;
    let output = imap::new_output(client, "feeds").await?;
    let syncer = sync::Syncer::new(&config.imap.name, &config.imap.email);
    let reporter = CliReporter::new()?;

    let urls = config.feeds.iter().map(|f| f.url.to_owned()).collect();
    syncer.sync(&urls, output, reporter).await?;

    Ok(())
}

async fn add_feed(cli: &Cli, args: &AddArgs) -> Result<(), Error> {
    let mut config = config::load(&cli.config_path())?;

    if config.feeds.iter().any(|feed| feed.url == args.url) {
        return Err(anyhow!("{} already in config", args.url));
    }

    log::info!("fetch {}", args.url);

    let _feed = fetch::url(&args.url).await?;

    config.feeds.push(config::Feed {
        url: args.url.to_owned(),
    });
    config::save(&config, &cli.config_path())?;

    Ok(())
}

async fn list_feeds(cli: &Cli) -> Result<(), Error> {
    let config = config::load(&cli.config_path())?;
    for feed in config.feeds {
        println!("Url: {}", feed.url);
    }
    Ok(())
}
