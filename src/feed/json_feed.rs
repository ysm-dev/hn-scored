use serde::Serialize;

use crate::feed::common::FeedView;

#[derive(Serialize)]
struct JsonFeed<'a> {
    version: &'static str,
    title: &'a str,
    home_page_url: &'static str,
    feed_url: &'a str,
    description: &'a str,
    items: Vec<JsonItem<'a>>,
}

#[derive(Serialize)]
struct JsonItem<'a> {
    id: &'a str,
    title: &'a str,
    url: &'a str,
    external_url: &'a str,
    content_text: &'a str,
    date_published: String,
    date_modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    authors: Option<Vec<JsonAuthor<'a>>>,
}

#[derive(Serialize)]
struct JsonAuthor<'a> {
    name: &'a str,
}

pub fn render(view: &FeedView) -> Vec<u8> {
    let feed = JsonFeed {
        version: "https://jsonfeed.org/version/1.1",
        title: &view.title,
        home_page_url: crate::config::HN_HOME_URL,
        feed_url: &view.self_url,
        description: &view.description,
        items: view
            .items
            .iter()
            .map(|item| JsonItem {
                id: &item.comments_url,
                title: &item.title,
                url: &item.link_url,
                external_url: &item.external_url,
                content_text: &item.summary,
                date_published: item.published.iso8601(),
                date_modified: item.modified.iso8601(),
                authors: (!item.author.is_empty()).then(|| vec![JsonAuthor { name: &item.author }]),
            })
            .collect(),
    };
    let mut bytes = serde_json::to_vec_pretty(&feed).expect("json feed serialization");
    bytes.push(b'\n');
    bytes
}
