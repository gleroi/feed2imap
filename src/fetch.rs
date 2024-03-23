use anyhow::{Context, Error};
use bytes::Buf;
use feed_rs::model::Feed;
use reqwest::ClientBuilder;

pub async fn url(url: &str) -> Result<Feed, Error> {
    let agent = ClientBuilder::new().build()?;
    let resp = agent
        .get(url)
        .send()
        .await
        .with_context(|| format!("could not fetch {}", url))?;
    let content = resp.bytes().await?;
    let feed = feed_rs::parser::parse(content.reader())
        .with_context(|| format!("could not parse {}", url))?;
    Ok(feed)
}
