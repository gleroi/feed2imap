use anyhow::Error;
use chrono::Utc;
use feed_rs::model::{Link, Person, Text};
use mail_builder::{
    headers::{address::Address, date::Date},
    mime::MimePart,
    MessageBuilder,
};
use reqwest::Url;
use std::fmt::Display;

mod html;

pub fn extract_message(
    name: &str,
    email: &str,
    full_feed: &feed_rs::model::Feed,
    entry: &feed_rs::model::Entry,
) -> Result<Vec<u8>, Error> {
    Ok(MessageBuilder::new()
        .message_id(extract_message_id(full_feed, entry))
        .from(Address::new_address(
            extract_feed_title(full_feed)?.into(),
            extract_email(full_feed, entry)?,
        ))
        .to(Address::new_address(name.into(), email))
        .date(extract_published_date(entry))
        .subject(extract_title(entry))
        .body(extract_content(entry)?)
        .write_to_vec()?)
}

fn extract_published_date(entry: &feed_rs::model::Entry) -> impl Into<Date> {
    entry
        .published
        .or_else(|| entry.updated)
        .unwrap_or_else(|| Utc::now())
        .timestamp()
}

pub fn extract_message_id(feed: &feed_rs::model::Feed, entry: &feed_rs::model::Entry) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&feed.id.as_bytes());
    hasher.update(&entry.id.as_bytes());
    let hash = hasher.finalize();
    return format!("{}", hash);
}

pub fn extract_feed_title(full_feed: &feed_rs::model::Feed) -> Result<String, Error> {
    Ok(full_feed.title.clone().unwrap_or(unknown_text()).content)
}

pub fn extract_email(
    feed: &feed_rs::model::Feed,
    entry: &feed_rs::model::Entry,
) -> Result<String, Error> {
    Ok(extract_authors(&feed.authors).unwrap_or_else(|| {
        extract_authors(&entry.authors).unwrap_or_else(|| {
            feed.links
                .first()
                .and_then(|l| {
                    let host = Url::parse(&l.href)
                        .ok()
                        .and_then(|url| url.host_str())
                        .unwrap_or("example.com");
                    Some(format!("rss@{}", host))
                })
                .unwrap_or_else(|| "placeholder@example.com".to_string())
        })
    }))
}

fn extract_authors(authors: &Vec<Person>) -> Option<String> {
    if let Some(ref author) = authors.iter().find(|author| author.email.is_some()) {
        Some(author.email.as_ref().unwrap().to_owned())
    } else {
        None
    }
}

pub fn extract_title(entry: &feed_rs::model::Entry) -> String {
    entry.title.clone().unwrap_or(unknown_text()).content
}

pub fn extract_content(entry: &feed_rs::model::Entry) -> Result<MimePart, Error> {
    let mut content = extract_atom_content(entry).or_else(|_| extract_rss_summary(entry))?;
    if let Some(ref base_url) = entry.base {
        content = html::rewrite_relative_link(base_url, content)?;
    }
    let link = extract_article_link(entry)?;
    content = wrap_content(content, link);
    // {
    //     let mut hasher = blake3::Hasher::new();
    //     hasher.update(&entry.id.as_bytes());
    //     let hash = hasher.finalize();
    //     let filename = format!("html/{}.html", hash);
    //     log::debug!("saving html of '{}' to {}", entry.id, filename);
    //     let mut debug = File::create(filename)?;
    //     debug.write_all(content.as_bytes())?;
    // }
    return Ok(MimePart::new("text/html", content));
}

pub fn extract_article_link(entry: &feed_rs::model::Entry) -> Result<Option<Link>, Error> {
    Ok(entry.links.first().and_then(|l| Some(l.to_owned())))
}

fn wrap_content(content: String, article_link: Option<Link>) -> String {
    let style = include_str!("../assets/message.css");
    let link_href = article_link
        .as_ref()
        .and_then(|l| Some(l.href.to_owned()))
        .unwrap_or("none".to_owned());
    let link_title = article_link
        .as_ref()
        .and_then(|l| l.title.to_owned())
        .unwrap_or(link_href.to_owned());

    format!(
        r#"
        <html>
            <head>
                <meta http-equiv="Content-Type" content="text/html">
                <style>
                    {style}
                </style>
            </head>
    
            <body>
                <div id="entry">
                    {content}
                </div>
                <div id=links>
                    <h4>Links</h4>
                    <ul>
                        <li><a href="{link_href}">{link_title}</a></li>
                    <ul>
                </div>
            </body>
        </html>
        "#,
        style = style,
        content = content,
        link_title = link_title,
        link_href = link_href
    )
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
