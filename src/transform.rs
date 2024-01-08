use anyhow::Error;
use feed_rs::model::Content;
use mail_builder::mime::MimePart;

pub fn extract_email(feed: &feed_rs::model::Feed) -> Result<String, Error> {
    return Ok("placeholder@example.org".to_string());
}

pub fn extract_content(entry: &feed_rs::model::Entry) -> Result<MimePart, Error> {
    return Ok(MimePart::new(
        "text/html",
        &entry
            .content
            .unwrap_or(unknown_content())
            .body
            .unwrap_or("unknown content".to_owned()),
    ));
}

fn unknown_content() -> Content {
    Content {
        content_type: mime::TEXT_PLAIN,
        body: "unknown content".to_owned().into(),
        length: None,
        src: None,
    }
}
