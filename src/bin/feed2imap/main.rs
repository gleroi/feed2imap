use anyhow::Error;
use clap::{Args, Parser, Subcommand};
use feed2imap::store::{DateTime, Entry, Utc};
use feed2imap::{fetch, store, transform};
use feed_rs::model::{Content, Text};
use mail_builder::headers::address::Address;
use mail_builder::MessageBuilder;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// database connection string "sqlite:path"
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// add a feed
    #[command()]
    Add(AddArgs),

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
        Command::List => list_feeds(&cli).await.unwrap(),
        Command::Sync => sync_feeds(&cli).await.unwrap(),
    }
}

async fn sync_feeds(cli: &Cli) -> Result<(), Error> {
    let store = store::Store::new(&cli.database_url).await?;
    let feeds = store.feeds().await?;

    for feed in feeds {
        log::info!("syncing {}", feed.title);
        let full_feed = fetch::url(&feed.url).await?;
        for entry in full_feed.entries {
            // println!("{:#?}", entry);
            let mail = MessageBuilder::new()
                .from(Address::new_address(
                    feed.title.into(),
                    transform::extract_email(&feed)?,
                ))
                .to(Address::new_address(
                    "Guillaume Leroi".into(),
                    "guillaume@leroi.re",
                ))
                .subject(&entry.title.unwrap_or(unknown_text()).content)
                .body(transform::extract_content(&entry)?)
                .html_body()
                .write_to(std::io::stdout())?;
            break; // TODO: remove it
        }
    }

    Ok(())
}

fn unknown_text() -> Text {
    Text {
        content_type: mime::TEXT_PLAIN,
        content: "Unknown".to_owned(),
        src: None,
    }
}

async fn add_feed(cli: &Cli, args: &AddArgs) -> Result<(), Error> {
    log::info!("fetch {}", args.url);

    let feed = fetch::url(&args.url).await?;
    let title = feed.title.clone().unwrap_or_else(|| unknown_text()).content;
    let updated = feed.updated.unwrap_or(Utc::now()).naive_utc();
    let checked = DateTime::default();

    let mut entries = Vec::with_capacity(feed.entries.len());
    for entry in feed.entries {
        entries.push(Entry {
            id: entry.id.clone(),
            feed_id: feed.id.clone(),
        });
    }

    log::debug!(
        "{:?} by {:?}, last updated on {:?} ({}) with {} entries",
        title,
        feed.authors,
        updated,
        feed.id,
        entries.len(),
    );

    let store = store::Store::new(&cli.database_url).await?;
    store
        .store_feed(
            &feed2imap::store::Feed {
                id: feed.id.clone(),
                title: title,
                url: args.url.clone(),
                last_updated: updated,
                last_checked: checked,
            },
            &entries,
        )
        .await?;

    Ok(())
}

async fn list_feeds(cli: &Cli) -> Result<(), Error> {
    let store = store::Store::new(&cli.database_url).await?;
    let feeds = store.feeds().await?;
    for feed in feeds {
        println!("Title: {}", feed.title);
        println!("Url: {}", feed.url);
        println!("Updated: {}", feed.last_updated);
        println!("Checked: {}", feed.last_checked);
        println!()
    }
    Ok(())
}
