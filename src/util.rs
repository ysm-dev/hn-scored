use html_escape::decode_html_entities;
use url::Url;

use crate::config::HN_ITEM_URL_PREFIX;

pub fn decode_title(value: &str) -> String {
    decode_html_entities(value).into_owned()
}

pub fn xml_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn description_domain(url: &str, id: u64) -> String {
    if url.is_empty() {
        return format!("news.ycombinator.com/item?id={id}");
    }
    let Ok(parsed) = Url::parse(url) else {
        return url.to_string();
    };
    let Some(host) = parsed.host_str() else {
        return url.to_string();
    };
    let host = host.strip_prefix("www.").unwrap_or(host);
    let port = parsed.port().filter(|_| {
        !matches!(
            (parsed.scheme(), parsed.port()),
            ("http", Some(80)) | ("https", Some(443))
        )
    });
    let mut value = host.to_string();
    if let Some(port) = port {
        value.push(':');
        value.push_str(&port.to_string());
    }
    if parsed.path() != "/" && !parsed.path().is_empty() {
        value.push_str(parsed.path());
    }
    value
}

pub fn hn_item_url(id: u64) -> String {
    format!("{HN_ITEM_URL_PREFIX}{id}")
}
