use crate::{
    config::{HN_HOME_URL, MAX_ITEMS_PER_FEED},
    types::{FeedFormat, LinkKind, State, Story},
    util::description_domain,
};

#[derive(Clone, Debug)]
pub struct FeedItemView {
    pub author: String,
    pub comments_url: String,
    pub external_url: String,
    pub link_url: String,
    pub modified: crate::time::Timestamp,
    pub published: crate::time::Timestamp,
    pub summary: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct FeedView {
    pub description: String,
    pub items: Vec<FeedItemView>,
    pub self_url: String,
    pub title: String,
    pub updated: crate::time::Timestamp,
}

pub fn build_view(
    state: &State,
    threshold: u16,
    kind: LinkKind,
    format: FeedFormat,
    base_url: &str,
) -> FeedView {
    let mut entries: Vec<_> = state
        .stories
        .values()
        .filter_map(|story| {
            story
                .thresholds
                .get(&threshold)
                .map(|published| (story, published.clone()))
        })
        .collect();
    entries.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.0.id.cmp(&left.0.id))
    });
    let items: Vec<_> = entries
        .into_iter()
        .take(MAX_ITEMS_PER_FEED)
        .map(|(story, published)| item_view(story, published, kind, threshold))
        .collect();
    let updated = items
        .iter()
        .map(|item| item.modified.clone())
        .max()
        .unwrap_or_else(crate::time::Timestamp::epoch);
    FeedView {
        description: feed_description(threshold, kind),
        items,
        self_url: format!("{base_url}/{}", feed_path(kind, format, threshold)),
        title: feed_title(threshold, kind),
        updated,
    }
}

pub fn feed_path(kind: LinkKind, format: FeedFormat, threshold: u16) -> String {
    format!("feeds/{}/{threshold}.{}", kind_dir(kind), extension(format))
}

pub fn headers_content() -> String {
    [
        "/feeds/article/*.xml\n  Content-Type: application/rss+xml; charset=utf-8\n  Cache-Control: public, max-age=60",
        "/feeds/comments/*.xml\n  Content-Type: application/rss+xml; charset=utf-8\n  Cache-Control: public, max-age=60",
        "/feeds/article/*.atom\n  Content-Type: application/atom+xml; charset=utf-8\n  Cache-Control: public, max-age=60",
        "/feeds/comments/*.atom\n  Content-Type: application/atom+xml; charset=utf-8\n  Cache-Control: public, max-age=60",
        "/feeds/article/*.json\n  Content-Type: application/feed+json; charset=utf-8\n  Cache-Control: public, max-age=60",
        "/feeds/comments/*.json\n  Content-Type: application/feed+json; charset=utf-8\n  Cache-Control: public, max-age=60",
        "/index.html\n  Cache-Control: public, max-age=60",
    ]
    .join("\n\n")
        + "\n"
}

pub fn mime_type(format: FeedFormat) -> &'static str {
    match format {
        FeedFormat::Rss => "application/rss+xml",
        FeedFormat::Atom => "application/atom+xml",
        FeedFormat::Json => "application/feed+json",
    }
}

fn item_view(
    story: &Story,
    published: crate::time::Timestamp,
    kind: LinkKind,
    threshold: u16,
) -> FeedItemView {
    let article_url = if story.url.is_empty() {
        story.hn_url.clone()
    } else {
        story.url.clone()
    };
    let (link_url, external_url) = match kind {
        LinkKind::Article => (article_url, story.hn_url.clone()),
        LinkKind::Comments => (story.hn_url.clone(), article_url),
    };
    let title = if threshold == 0 || story.story_time <= 0 {
        story.title.clone()
    } else {
        title_with_elapsed(story, &published)
    };
    FeedItemView {
        author: story.by.clone(),
        comments_url: story.hn_url.clone(),
        external_url,
        link_url,
        modified: story.last_output_change_at.clone(),
        published,
        summary: format!(
            "{} points | {} comments | {}",
            story.score,
            story.comments,
            description_domain(&story.url, story.id)
        ),
        title,
    }
}

/// Appends an elapsed-time suffix to the story title, e.g. "Title (2d 3h 32m)".
/// `d`/`h` segments are omitted when zero; `m` is always shown.
fn title_with_elapsed(story: &Story, published: &crate::time::Timestamp) -> String {
    let total_minutes = published.minutes_since(story.story_time);
    let days = total_minutes / (24 * 60);
    let hours = (total_minutes % (24 * 60)) / 60;
    let minutes = total_minutes % 60;
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    parts.push(format!("{minutes}m"));
    format!("{} ({})", story.title, parts.join(" "))
}

fn feed_title(threshold: u16, kind: LinkKind) -> String {
    match (threshold, kind) {
        (0, LinkKind::Article) => "Hacker News - All Stories".to_string(),
        (0, LinkKind::Comments) => "Hacker News - All Stories (comments)".to_string(),
        (_, LinkKind::Article) => format!("Hacker News - {threshold}+ points"),
        (_, LinkKind::Comments) => format!("Hacker News - {threshold}+ points (comments)"),
    }
}

fn feed_description(threshold: u16, kind: LinkKind) -> String {
    match (threshold, kind) {
        (0, LinkKind::Article) => "All Hacker News stories".to_string(),
        (0, LinkKind::Comments) => "All Hacker News stories (links to comments)".to_string(),
        (_, LinkKind::Article) => format!("Hacker News stories with {threshold} or more points"),
        (_, LinkKind::Comments) => {
            format!("Hacker News stories with {threshold} or more points (links to comments)")
        }
    }
}

fn extension(format: FeedFormat) -> &'static str {
    match format {
        FeedFormat::Rss => "xml",
        FeedFormat::Atom => "atom",
        FeedFormat::Json => "json",
    }
}

fn kind_dir(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::Article => "article",
        LinkKind::Comments => "comments",
    }
}

pub fn site_url() -> &'static str {
    HN_HOME_URL
}
