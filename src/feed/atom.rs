use crate::{
    feed::common::{FeedView, site_url},
    util::xml_escape,
};

pub fn render(view: &FeedView) -> Vec<u8> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">\n",
    );
    xml.push_str(&format!("  <title>{}</title>\n", xml_escape(&view.title)));
    xml.push_str(&format!(
        "  <link href=\"{}\" rel=\"alternate\"/>\n",
        site_url()
    ));
    xml.push_str(&format!(
        "  <link href=\"{}\" rel=\"self\"/>\n",
        xml_escape(&view.self_url)
    ));
    xml.push_str(&format!("  <id>{}</id>\n", xml_escape(&view.self_url)));
    xml.push_str(&format!(
        "  <updated>{}</updated>\n",
        view.updated.iso8601()
    ));
    xml.push_str(&format!(
        "  <subtitle>{}</subtitle>\n",
        xml_escape(&view.description)
    ));
    xml.push_str("  <generator>hn-scored</generator>\n");
    for item in &view.items {
        xml.push_str("  <entry>\n");
        xml.push_str(&format!("    <title>{}</title>\n", xml_escape(&item.title)));
        xml.push_str(&format!(
            "    <link href=\"{}\" rel=\"alternate\"/>\n",
            xml_escape(&item.link_url)
        ));
        xml.push_str(&format!(
            "    <id>{}</id>\n",
            xml_escape(&item.comments_url)
        ));
        xml.push_str(&format!(
            "    <updated>{}</updated>\n",
            item.modified.iso8601()
        ));
        xml.push_str(&format!(
            "    <published>{}</published>\n",
            item.published.iso8601()
        ));
        if !item.author.is_empty() {
            xml.push_str(&format!(
                "    <author><name>{}</name></author>\n",
                xml_escape(&item.author)
            ));
        }
        xml.push_str(&format!(
            "    <summary>{}</summary>\n",
            xml_escape(&item.summary)
        ));
        xml.push_str("  </entry>\n");
    }
    xml.push_str("</feed>\n");
    xml.into_bytes()
}
