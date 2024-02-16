use std::{collections::BTreeSet, sync::Arc};

use anyhow::{anyhow, Error};
use clap::{Args, Parser, Subcommand};
use feed2imap::{fetch, imap, transform};
use futures::future::try_join_all;
use tokio::sync::Mutex;

pub mod config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// path to configuration file
    #[arg(long, env = "FEED2IMAP_CONFIG")]
    config: String,

    #[command(subcommand)]
    command: Command,
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
    let config = Arc::new(config::load(&cli.config)?);
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

    let futures: Vec<_> = config
        .feeds
        .iter()
        .map(|feed| {
            let inner_ids = ids.clone();
            let url = feed.url.clone();
            let inner_config = config.clone();
            let inner_imap = imap_client.clone();
            tokio::spawn(async {
                sync_feed(url, inner_ids, inner_config, inner_imap).await?;
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
) -> Result<(), Error> {
    log::info!("syncing {}", url);
    let full_feed = fetch::url(&url).await?;
    Ok(for entry in &full_feed.entries {
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
    })
}

async fn add_feed(cli: &Cli, args: &AddArgs) -> Result<(), Error> {
    let mut config = config::load(&cli.config)?;

    if config.feeds.iter().any(|feed| feed.url == args.url) {
        return Err(anyhow!("{} already in config", args.url));
    }

    log::info!("fetch {}", args.url);

    let _feed = fetch::url(&args.url).await?;

    config.feeds.push(config::Feed {
        url: args.url.to_owned(),
    });
    config::save(&config, &cli.config)?;

    Ok(())
}

async fn list_feeds(cli: &Cli) -> Result<(), Error> {
    let config = config::load(&cli.config)?;
    for feed in config.feeds {
        println!("Url: {}", feed.url);
    }
    Ok(())
}
