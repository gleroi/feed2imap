use anyhow::Error;
use bytes::buf::Buf;
use clap::{Args, Parser, Subcommand};
use feed2imap::store;
use feed2imap::store::Utc;
use feed_rs::model::Text;
use reqwest::{header::HeaderValue, ClientBuilder};

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
    let agent = ClientBuilder::new().build()?;
    let resp = agent.get(&args.url).send().await?;
    log::debug!(
        "Content-Type: {}",
        resp.headers()
            .get("Content-Type")
            .unwrap_or(&HeaderValue::from_str("none").unwrap())
            .to_str()?
    );
    let content = resp.bytes().await?;
    let feed = feed_rs::parser::parse(content.reader())?;

    let store = store::Store::new(&std::env::var("DATABASE_URL")?).await?;
    let title = feed.title.clone().unwrap_or_else(|| unknown_text()).content;
    let updated = feed.updated.unwrap_or(Utc::now());
    let checked = Utc::now();

    log::debug!(
        "{:?} by {:?}, last updated on {:?} ({})",
        title,
        feed.authors,
        updated,
        feed.id,
    );

    store
        .add_feed(feed2imap::store::Feed {
            id: feed.id,
            title: title,
            last_updated: updated,
            last_checked: checked,
        })
        .await?;

    for entry in feed.entries {
        log::debug!(
            "- {} ({})",
            entry.title.unwrap_or_else(|| unknown_text()).content,
            entry.id
        );
    }
    Ok(())
}
