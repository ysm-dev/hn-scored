use crate::{
    feed::common::{FeedView, mime_type, site_url},
    util::xml_escape,
};

pub fn render(view: &FeedView) -> Vec<u8> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n  <channel>\n",
    );
    xml.push_str(&format!("    <title>{}</title>\n", xml_escape(&view.title)));
    xml.push_str(&format!("    <link>{}</link>\n", site_url()));
    xml.push_str(&format!(
        "    <description>{}</description>\n",
        xml_escape(&view.description)
    ));
    xml.push_str(&format!(
        "    <lastBuildDate>{}</lastBuildDate>\n",
        view.updated.rfc2822()
    ));
    xml.push_str("    <ttl>1</ttl>\n    <generator>hn-scored</generator>\n");
    xml.push_str(&format!(
        "    <atom:link href=\"{}\" rel=\"self\" type=\"{}\"/>\n",
        xml_escape(&view.self_url),
        mime_type(crate::types::FeedFormat::Rss)
    ));
    for item in &view.items {
        xml.push_str("    <item>\n");
        xml.push_str(&format!(
            "      <title>{}</title>\n",
            xml_escape(&item.title)
        ));
        xml.push_str(&format!(
            "      <link>{}</link>\n",
            xml_escape(&item.link_url)
        ));
        xml.push_str(&format!(
            "      <guid isPermaLink=\"false\">{}</guid>\n",
            xml_escape(&item.comments_url)
        ));
        xml.push_str(&format!(
            "      <pubDate>{}</pubDate>\n",
            item.published.rfc2822()
        ));
        xml.push_str(&format!(
            "      <description>{}</description>\n",
            xml_escape(&item.summary)
        ));
        xml.push_str(&format!(
            "      <comments>{}</comments>\n",
            xml_escape(&item.comments_url)
        ));
        xml.push_str("    </item>\n");
    }
    xml.push_str("  </channel>\n</rss>\n");
    xml.into_bytes()
}
