use anyhow::Error;
use clap::{Args, Parser, Subcommand};
use feed2imap::store::{DateTime, Entry, Utc};
use feed2imap::{fetch, store};
use feed_rs::model::Text;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// add a feed
    #[command()]
    Add(AddArgs),
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
    }
}

fn unknown_text() -> Text {
    Text {
        content_type: mime::TEXT_PLAIN,
        content: "Unknown".to_owned(),
        src: None,
    }
}

async fn add_feed(_cli: &Cli, args: &AddArgs) -> Result<(), Error> {
    log::info!("fetch {}", args.url);

    let feed = fetch::url(&args.url).await?;
    let store = store::Store::new(&std::env::var("DATABASE_URL")?).await?;
    let title = feed.title.clone().unwrap_or_else(|| unknown_text()).content;
    let updated = feed.updated.unwrap_or(Utc::now());
    let checked = DateTime::<Utc>::default();

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
