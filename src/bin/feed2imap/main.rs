use std::{collections::BTreeSet, sync::Arc};

use anyhow::{anyhow, Error};
use clap::{Args, Parser, Subcommand};
use directories::BaseDirs;
use feed2imap::{fetch, imap, transform};
use futures::future::try_join_all;
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

async fn sync_feeds(cli: &Cli) -> Result<(), Error> {
    let config = Arc::new(config::load(&cli.config_path())?);
    log::debug!("connecting to mail server");
    let imap_client = Arc::new(Mutex::new(
        imap::client(&config.imap.username, &config.imap.password).await?,
    ));

    log::debug!("get email ids");
    let ids = {
        let mut imap_guard = imap_client.lock().await;
        Arc::new(imap_guard.list_message_ids("feeds").await?)
    };

    log::debug!("{} emails found", ids.len());

    let multi_bar = MultiProgress::new();
    let style = ProgressStyle::default_bar()
        .progress_chars("##-")
        .template("[{prefix:20}] {wide_bar} [{pos:>4} / {len:4}]")?;

    let futures: Vec<_> = config
        .feeds
        .iter()
        .map(|feed| {
            let inner_ids = ids.clone();
            let url = feed.url.clone();
            let inner_config = config.clone();
            let inner_imap = imap_client.clone();
            let pb = multi_bar.add(ProgressBar::new(0));
            pb.set_style(style.clone());
            tokio::spawn(async {
                sync_feed(url, inner_ids, inner_config, inner_imap, pb).await?;
                Ok::<(), Error>(())
            })
        })
        .collect();
    let _results = try_join_all(futures).await?;
    Ok(())
}

async fn sync_feed(
    url: String,
    ids: Arc<BTreeSet<String>>,
    config: Arc<config::Config>,
    imap_lock: Arc<Mutex<imap::Client>>,
    pb: ProgressBar,
) -> Result<(), Error> {
    log::info!("syncing {}", url);
    let full_feed = fetch::url(&url).await?;

    let title: String = transform::extract_feed_title(&full_feed)?
        .chars()
        .take(20)
        .collect();
    pb.set_prefix(title);
    pb.set_length(full_feed.entries.len() as u64);

    for entry in &full_feed.entries {
        let id = transform::extract_message_id(&full_feed, &entry);
        if !ids.contains(&id) {
            let mail = transform::extract_message(
                &config.imap.name,
                &config.imap.email,
                &full_feed,
                entry,
            )?;
            log::debug!("{}: {} appending to mail", url, id);
            let mut imap_client = imap_lock.lock().await;
            imap_client.append(&mail, "feeds").await?;
            log::debug!("{}: {} appended to mail", url, id);
        } else {
            log::debug!("{}: {} already in mail", url, id);
        }
        pb.inc(1);
    }
    pb.finish();
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
