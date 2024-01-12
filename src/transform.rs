use std::{fmt::Display, sync::Arc};

use anyhow::Error;
use feed_rs::model::{Content, Text};
use mail_builder::{headers::address::Address, mime::MimePart, MessageBuilder};

pub fn extract_message(
    full_feed: &feed_rs::model::Feed,
    entry: &feed_rs::model::Entry,
) -> Result<Vec<u8>, Error> {
    Ok(MessageBuilder::new()
        .message_id(extract_message_id(full_feed, entry))
        .from(Address::new_address(
            full_feed
                .title
                .clone()
                .unwrap_or(unknown_text())
                .content
                .into(),
            extract_email(full_feed)?,
        ))
        .to(Address::new_address(
            "Guillaume Leroi".into(),
            "guillaume@leroi.re",
        ))
        .subject(extract_title(entry))
        .body(extract_content(entry)?)
        .write_to_vec()?)
}

pub fn extract_message_id(feed: &feed_rs::model::Feed, entry: &feed_rs::model::Entry) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&feed.id.as_bytes());
    hasher.update(&entry.id.as_bytes());
    let hash = hasher.finalize();
    return format!("{}", hash);
}

pub fn extract_email(feed: &feed_rs::model::Feed) -> Result<String, Error> {
    return Ok("placeholder@example.org".to_string());
}

pub fn extract_title(entry: &feed_rs::model::Entry) -> String {
    entry.title.clone().unwrap_or(unknown_text()).content
}

pub fn extract_content(entry: &feed_rs::model::Entry) -> Result<MimePart, Error> {
    let content = extract_atom_content(entry).or_else(|_| extract_rss_summary(entry))?;
    return Ok(MimePart::new("text/html", content));
}

fn extract_atom_content(entry: &feed_rs::model::Entry) -> Result<String, Error> {
    Ok(entry
        .content
        .clone()
        .ok_or(NO_CONTENT)?
        .body
        .ok_or(NO_CONTENT)?)
}

fn extract_rss_summary(entry: &feed_rs::model::Entry) -> Result<String, Error> {
    Ok(entry.summary.clone().ok_or(NO_CONTENT)?.content)
}

#[derive(Debug, Clone, Copy)]
struct NoContent;

impl Display for NoContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no content")
    }
}

impl std::error::Error for NoContent {}

const NO_CONTENT: NoContent = NoContent {};

fn unknown_text() -> Text {
    Text {
        content_type: mime::TEXT_PLAIN,
        content: "Unknown".to_owned(),
        src: None,
    }
}