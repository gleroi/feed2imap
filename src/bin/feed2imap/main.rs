use anyhow::Error;
use clap::{Args, Parser, Subcommand};
use feed2imap::{fetch, imap, transform};
use feed_rs::model::Text;

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
    let config = config::load(&cli.config)?;
    log::debug!("connecting to mail server");
    let mut imap_client = imap::client(&config.imap.username, &config.imap.password).await?;
    log::debug!("get email ids");
    let ids = imap_client.list_message_ids("feeds").await?;
    log::debug!("{} emails found", ids.len());

    for feed in &config.feeds {
        log::info!("syncing {}", feed.url);
        let full_feed = fetch::url(&feed.url).await?;
        for entry in &full_feed.entries {
            let id = transform::extract_message_id(&full_feed, &entry);
            if !ids.contains(&id) {
                let mail = transform::extract_message(&full_feed, entry)?;
                imap_client.append(&mail, "feeds").await?;
                log::debug!("{} appended to mail", id);
            } else {
                log::debug!("{} already in mail", id);
            }
        }
    }

    imap_client.logout().await?;
    Ok(())
}

async fn add_feed(cli: &Cli, args: &AddArgs) -> Result<(), Error> {
    let mut config = config::load(&cli.config)?;

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
