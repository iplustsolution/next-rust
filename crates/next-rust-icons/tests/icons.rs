use next_rust_icons::{self as icons, Icon, IconData};
use next_rust_view::*;

#[test]
fn every_lucide_icon_is_here_sorted_and_well_formed() {
    let all = icons::all();
    assert!(all.len() >= 1800, "{}", all.len());
    assert!(all.windows(2).all(|w| w[0].name < w[1].name), "sorted and unique");
    for icon in all {
        assert!(icon.name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'), "{}", icon.name);
        assert!(icon.body.starts_with('<') && icon.body.ends_with("/>"), "{}", icon.name);
        for tag in icon.body.split('<').skip(1).map(|e| e.split([' ', '/']).next().unwrap()) {
            assert!(["path", "circle", "rect", "line", "ellipse", "polyline", "polygon"].contains(&tag), "{tag}");
        }
    }
    assert!(!icons::LUCIDE_VERSION.is_empty());
}

#[test]
fn renders_like_lucide() {
    let html = render_static(icons::House());
    assert_eq!(
        html,
        r#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-house" aria-hidden="true"><path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"/><path d="M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>"#
    );
}

#[test]
fn everything_is_customizable() {
    let html = render_static(
        icons::ArrowRight()
            .size(48)
            .color("var(--brand)")
            .stroke_width(2.0)
            .absolute_stroke_width(true)
            .fill("#fff")
            .class("h-4 w-4")
            .with(attr("stroke-linecap", "square"))
            .with(data("kind", "nav")),
    );
    for part in [
        r#"width="48" height="48""#,
        r##"fill="#fff""##,
        r#"stroke="var(--brand)""#,
        r#"stroke-width="1""#,
        r#"stroke-linecap="square""#,
        r#"class="lucide lucide-arrow-right h-4 w-4""#,
        r#"data-kind="nav""#,
    ] {
        assert!(html.contains(part), "{part} in {html}");
    }
    let em = render_static(icons::Check().size("1.25em").absolute_stroke_width(true));
    assert!(em.contains(r#"width="1.25em""#) && em.contains(r#"stroke-width="2""#), "no pixel size: stroke as is");
    let bare = render_static(icons::Check().unstyled(true).class("mine"));
    assert!(bare.contains(r#"class="mine""#) && !bare.contains("lucide"));
}

#[test]
fn titles_make_icons_accessible_and_text_is_escaped() {
    let html = render_static(icons::Trash2().title("Delete <all>").color("\"red"));
    assert!(html.contains(r#"role="img" aria-label="Delete &lt;all&gt;""#), "{html}");
    assert!(html.contains("<title>Delete &lt;all&gt;</title>"));
    assert!(html.contains(r#"stroke="&quot;red""#));
    assert!(!html.contains("aria-hidden"));
}

#[test]
fn lookup_by_name_and_macro() {
    assert_eq!(icons::by_name("arrow-down-0-1").map(|i| i.name()), Some("arrow-down-0-1"));
    // Former names still work, as functions and by name.
    assert_eq!(icons::by_name("home").map(|i| i.name()), Some("house"));
    assert_eq!(icons::Trash2().name(), "trash");
    assert_eq!(icons::Home().name(), "house");
    assert!(icons::by_name("no-such-icon").is_none());
    let a = render_static(icons::by_name("house").unwrap().size(20));
    let b = render_static(next_rust_icons::Icon![House, size = 20]);
    assert_eq!(a, b);
    let c = render_static(next_rust_icons::Icon![Search, class("x"), title = "Find"]);
    assert!(c.contains("lucide-search x") && c.contains("<title>Find</title>"));
    // Custom drawings work the same way.
    static LOGO: IconData = IconData { name: "logo", body: r#"<circle cx="12" cy="12" r="10"/>"# };
    assert!(render_static(Icon::new(&LOGO)).contains(r#"class="lucide lucide-logo""#));
}
