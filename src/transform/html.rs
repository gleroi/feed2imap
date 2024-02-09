use anyhow::Error;
use lol_html::{element, html_content::Element, rewrite_str, HandlerResult, RewriteStrSettings};
use reqwest::Url;

pub fn rewrite_relative_link(base_url: &str, content: String) -> Result<String, Error> {
    let url = Url::parse(base_url)?;
    let element_content_handlers = vec![element!("img[src]", |el| rewrite_relative_src(&url, el))];
    let rewritten_content = rewrite_str(
        &content,
        RewriteStrSettings {
            element_content_handlers,
            ..RewriteStrSettings::default()
        },
    )?;
    Ok(rewritten_content)
}

fn rewrite_relative_src(base_url: &Url, el: &mut Element) -> HandlerResult {
    if let Some(mut src) = el.get_attribute("src") {
        src = html_escape::decode_html_entities(&src).to_string();
        let options = Url::options().base_url(Some(base_url));
        match options.parse(&src) {
            Ok(url) => {
                el.set_attribute("src", url.as_str())?;
            }
            Err(err) => {
                log::error!("could not parse src url {}: {}", src, err);
            }
        }
    }
    Ok(())
}
