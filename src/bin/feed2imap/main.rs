use anyhow::{anyhow, Error};
use clap::{Args, Parser, Subcommand};
use directories::BaseDirs;
use feed2imap::{fetch, imap, sync, transform};
use std::sync::Arc;

use crate::reporter::{CliReporter, SimpleReporter};

pub mod config;
pub mod reporter;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// path to configuration file, default to ~/.config/feed2imap.toml
    #[arg(long, env = "FEED2IMAP_CONFIG")]
    config: Option<String>,

    /// enable batch mode, i.e simple logging to stdout
    #[arg(long, default_value_t = false)]
    batch: bool,

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

    let result = match &cli.command {
        Command::Add(ref args) => add_feed(&cli, args).await,
        Command::Config => config(&cli).await,
        Command::List => list_feeds(&cli).await,
        Command::Sync => sync_feeds(&cli).await,
    };
    if let Err(err) = result {
        eprintln!("ERROR: {}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}

async fn config(_cli: &Cli) -> Result<(), Error> {
    config::dump_default()
}

async fn sync_feeds(cli: &Cli) -> Result<(), Error> {
    let config = Arc::new(config::load(&cli.config_path())?);
    log::debug!("connecting to mail server");
    let client = imap::client(&config.imap.username, &config.imap.password).await?;
    let output = imap::new_output(client, &config.imap.default_folder).await?;
    let syncer = sync::Syncer::new(&config.imap.name, &config.imap.email);
    if cli.batch {
        let reporter = SimpleReporter {};
        syncer.sync(&config.feeds, output, reporter).await?;
    } else {
        let reporter = CliReporter::new()?;
        syncer.sync(&config.feeds, output, reporter).await?;
    };

    Ok(())
}

async fn add_feed(cli: &Cli, args: &AddArgs) -> Result<(), Error> {
    let mut config = config::load(&cli.config_path())?;

    log::info!("fetch {}", args.url);

    let feed = fetch::url(&args.url).await?;
    let title = transform::extract_feed_title(&feed)?;
    let email = transform::extract_email(&feed, feed.entries.first().expect("no entries in feed"))?;

    println!("Title: {}\nEmail: {}\nUrl: {}", title, email, args.url);

    if config.feeds.iter().any(|feed| feed.url == args.url) {
        return Err(anyhow!("{} already in config", args.url));
    }

    config.feeds.push(config::Feed {
        url: args.url.to_owned(),
    });
    config::save(&config, &cli.config_path())?;

    Ok(())
}

async fn list_feeds(cli: &Cli) -> Result<(), Error> {
    let config = config::load(&cli.config_path())?;
    for feed in config.feeds {
        let full_feed = fetch::url(&feed.url).await?;
        let title = transform::extract_feed_title(&full_feed)?;
        let email = transform::extract_email(
            &full_feed,
            full_feed.entries.first().expect("no entries in feed"),
        )?;
        println!("Title: {}\nEmail: {}\nUrl: {}\n", title, email, feed.url);
    }
    Ok(())
}
