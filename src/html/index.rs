use crate::{
    config::THRESHOLDS,
    feed::common::feed_path,
    time::Timestamp,
    types::{FeedFormat, LinkKind},
};

pub fn render(base_url: &str, last_change: &Timestamp) -> Vec<u8> {
    let rows = THRESHOLDS
        .iter()
        .map(|threshold| row(base_url, *threshold))
        .collect::<Vec<_>>()
        .join("\n");
    [
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
        "<title>hn-scored</title><style>body{margin:0;background:#f6f6ef;color:#222;font:14px/1.4 SFMono-Regular,Menlo,Consolas,monospace}header{background:#ff6600;padding:10px 14px;font-weight:700}main{padding:14px}table{width:100%;border-collapse:collapse}th,td{padding:8px;vertical-align:top;border-bottom:1px solid #e5d7b5;text-align:left}code{word-break:break-all}button{margin-left:6px}footer{margin-top:16px;font-size:13px}</style></head><body>",
        "<header>hn-scored</header><main><p>Hacker News stories filtered by score. Pick a threshold and subscribe.</p>",
        &format!("<table><thead><tr><th>Threshold</th><th>Article</th><th>Comments</th></tr></thead><tbody>{rows}</tbody></table>"),
        &format!("<footer>Last feed change: <code>{}</code> | <a href=\"{}\">GitHub</a></footer>", last_change.iso8601(), crate::config::REPOSITORY_URL),
        "</main><script>function copyUrl(url){navigator.clipboard.writeText(url)}</script></body></html>",
    ]
    .join("")
    .into_bytes()
}

fn row(base_url: &str, threshold: u16) -> String {
    format!(
        "<tr><td>{threshold}</td><td>{}</td><td>{}</td></tr>",
        link_group(base_url, threshold, LinkKind::Article),
        link_group(base_url, threshold, LinkKind::Comments),
    )
}

fn link_group(base_url: &str, threshold: u16, kind: LinkKind) -> String {
    [FeedFormat::Rss, FeedFormat::Atom, FeedFormat::Json]
        .into_iter()
        .map(|format| {
            let url = format!("{base_url}/{}", feed_path(kind, format, threshold));
            format!("<div>{label}: <code>{url}</code><button type=\"button\" onclick=\"copyUrl('{url}')\">Copy</button></div>", label = label(format), url = url)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn label(format: FeedFormat) -> &'static str {
    match format {
        FeedFormat::Rss => "RSS",
        FeedFormat::Atom => "Atom",
        FeedFormat::Json => "JSON",
    }
}
