use hn_scored::{html, time::Timestamp};

#[test]
fn landing_page_uses_state_timestamp_and_lists_thresholds() {
    let html = String::from_utf8(html::index::render(
        "https://hn.ysm.dev",
        &Timestamp::parse("2025-04-14T12:34:56Z").unwrap(),
    ))
    .unwrap();
    assert!(html.contains("Last feed change: <code>2025-04-14T12:34:56Z</code>"));
    assert!(html.find("<td>0</td>").unwrap() < html.find("<td>1000</td>").unwrap());
    assert!(html.contains("navigator.clipboard.writeText"));
}
