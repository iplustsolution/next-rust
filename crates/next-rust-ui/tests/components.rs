use std::collections::BTreeSet;

use next_rust_ui::*;
use next_rust_view::*;

fn html(view: impl View) -> String {
    render_static(view)
}

/// The markup without the inline `<style>` of `render_static`.
fn markup(view: impl View) -> String {
    let h = html(view);
    let mut out = String::new();
    let mut rest = h.as_str();
    while let Some(at) = rest.find("<style") {
        out.push_str(&rest[..at]);
        rest = &rest[at + rest[at..].find("</style>").unwrap() + "</style>".len()..];
    }
    out + rest
}

#[test]
fn classes_merge_and_can_be_dropped() {
    let b = markup(Button![class("w-full"), class = "shadow-xl", id("save"), "Save"]);
    assert!(b.contains(r#"class="nr-btn nr-btn-solid nr-c-primary nr-btn-md w-full shadow-xl""#), "{b}");
    assert!(b.contains(r#"id="save""#));
    let bare = markup(Button![unstyled = true, class("my-button"), "Plain"]);
    assert!(bare.starts_with(r#"<button class="my-button" type="button">"#), "{bare}");
}

#[test]
fn text_is_escaped() {
    let h = markup(Input![label = "<b>x</b>", placeholder = "\"quoted\"", value = "a<b"]);
    assert!(h.contains("&lt;b&gt;x&lt;/b&gt;") && h.contains("&quot;quoted&quot;") && h.contains(r#"value="a&lt;b""#));
}

#[test]
fn fields_link_labels_and_messages() {
    let h = markup(Input![
        id = "email",
        name = "email",
        label = "Email",
        description = "Work address",
        error_message = Some("Required")
    ]);
    assert!(h.contains(r#"<label class="nr-field-label" for="email" id="email-label">Email</label>"#), "{h}");
    assert!(h.contains(r#"aria-describedby="email-description email-error""#));
    assert!(h.contains(r#"aria-invalid="true""#));
    assert!(h.contains(r#"data-nr-error="email">Required</p>"#));
    // Attributes other than `class` reach the control.
    let h = markup(Input![attr("autocomplete", "email"), class("outer"), name = "e"]);
    assert!(h.contains(r#"autocomplete="email""#) && h.contains("nr-field-full outer"), "{h}");
    assert!(h.find("autocomplete").unwrap() > h.find("<input").unwrap());
    // No error and no name: no error element at all.
    assert!(!markup(Input![label = "Plain"]).contains("nr-field-error"));
    // `error_message = form.error(..)` with `None` means valid.
    assert!(!markup(Input![name = "x", error_message = None::<&str>]).contains("aria-invalid"));
}

#[test]
fn password_select_and_date_picker_work_without_script() {
    let pw = markup(PasswordInput![name = "password", new_password = true]);
    assert!(pw.contains(r#"type="password""#) && pw.contains(r#"autocomplete="new-password""#));
    assert!(pw.contains(r#"data-nr-field-action="toggle-password""#));

    let s = markup(Select![name = "c", required = true, placeholder = "Pick", [("a", "A"), ("b", "B")]]);
    assert!(s.contains(r#"<option value="" selected disabled>Pick</option>"#), "{s}");
    assert!(s.contains(r#"<select class="nr-field-input nr-select-native" id="#));
    let multi = markup(Select![multiple = true, values = ["b"], [("a", "A"), ("b", "B")]]);
    assert!(multi.contains(r#"<option value="b" selected>B</option>"#) && multi.contains("multiple"));
    assert!(!multi.contains(r#"<option value="">"#), "no empty choice for multiple selects");

    let d = markup(DatePicker![name = "day", value = "2026-09-18", min = "2026-01-01", first_day_of_week = 8]);
    assert!(d.contains(r#"type="date""#) && d.contains(r#"value="2026-09-18""#) && d.contains(r#"min="2026-01-01""#));
    assert!(d.contains(r#"data-first-day="1""#), "wraps to a weekday");
}

#[test]
fn press_and_links() {
    let h = markup(Button![on_press = Press::action("/_nr/action/x").input(&42), "Go"]);
    assert!(
        h.contains(r#"data-nr-press="action" data-nr-press-target="/_nr/action/x" data-nr-press-input="42""#),
        "{h}"
    );
    assert!(h.contains("nr-btn-pending"), "pending spinner for actions");
    let h = markup(Button![on_press = Press::navigate("/a").no_refresh(), "Go"]);
    assert!(h.contains(r#"data-nr-press="navigate""#) && h.contains(r#"data-nr-press-refresh="false""#));
    let link = markup(Button![href = "/docs", disabled = true, "Docs"]);
    assert!(
        link.starts_with(r#"<a class="nr-btn"#)
            && link.contains(r#"aria-disabled="true""#)
            && link.contains("data-nr-link")
    );
    let card = markup(Card![on_press = Press::emit("open"), "x"]);
    assert!(card.contains(r#"role="button" tabindex="0""#));
}

#[test]
fn groups_pass_props_down() {
    let r = markup(RadioGroup![
        name = "plan",
        value = "b",
        color = Color::Success,
        Radio![value = "a", "A"],
        Radio![value = "b", "B"]
    ]);
    assert_eq!(r.matches(r#"name="plan""#).count(), 2);
    assert_eq!(r.matches("checked").count(), 1);
    assert_eq!(r.matches("nr-c-success").count(), 2);
    let g = markup(AvatarGroup![max = 2, Avatar![name = "A"], Avatar![name = "B"], Avatar![name = "C"]]);
    assert!(g.contains(">+1<") && g.contains(r#"aria-label="1 more""#), "{g}");
}

/// Classes in `class` and `data-nr-active` attributes of `html`.
fn classes(html: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for attr in ["class=\"", "data-nr-active=\""] {
        for part in html.split(attr).skip(1) {
            out.extend(part.split('"').next().unwrap().split_whitespace().map(str::to_owned));
        }
    }
    out
}

/// Every rule of the stylesheet starts with a class the components render.
/// Pages only receive the rules whose first class is on the page, so a rule
/// keyed on anything else would silently never be sent.
#[test]
fn every_css_rule_belongs_to_rendered_markup() {
    let colors = [Color::Default, Color::Primary, Color::Secondary, Color::Success, Color::Warning, Color::Danger];
    let sizes = [Size::Sm, Size::Md, Size::Lg];
    let variants = [
        Variant::Solid,
        Variant::Bordered,
        Variant::Light,
        Variant::Flat,
        Variant::Faded,
        Variant::Shadow,
        Variant::Ghost,
    ];
    let fields = [FieldVariant::Flat, FieldVariant::Bordered, FieldVariant::Faded, FieldVariant::Underlined];
    let placements = [LabelPlacement::Inside, LabelPlacement::Outside, LabelPlacement::OutsideLeft];
    let radii = [Radius::None, Radius::Sm, Radius::Md, Radius::Lg, Radius::Full];
    let mut all = String::new();
    for c in colors {
        for v in variants {
            all += &markup(Button![color = c, variant = v, "x"]);
            all += &markup(Chip![color = c, variant = v, dot = true, "x"]);
        }
        all += &markup(Spinner![color = c]);
    }
    for s in sizes {
        all += &markup(Button![size = s, full_width = true, icon_only = true, loading = true, "x"]);
        all += &markup(Chip![size = s, "x"]);
        all += &markup(Spinner![size = s, label = "x"]);
        all += &markup(Avatar![size = s, src = "/a.png", name = "A B", bordered = true]);
        all += &markup(Checkbox![size = s, description = "d", "x"]);
        all += &markup(Switch![size = s, "x"]);
        all += &markup(RadioGroup![size = s, horizontal = true, name = "r", Radio![value = "a", "A"]]);
        for f in fields {
            for p in placements {
                all += &markup(Input![
                    size = s,
                    variant = f,
                    label_placement = p,
                    label = "L",
                    required = true,
                    clearable = true,
                    description = "d",
                    name = "n"
                ]);
            }
        }
    }
    for r in radii {
        all += &markup(Button![radius = r, "x"]);
    }
    all += &markup(ButtonGroup![full_width = true, Button!["a"], Button![submit = true, "b"]]);
    all += &markup(PasswordInput![label = "p", start_content = "s"]);
    all += &markup(Textarea![label = "t"]);
    all += &markup(Select![label = "s", SelectItem![start_content = "i", description = "d", "A"]]);
    all += &markup(DatePicker![label = "d"]);
    all += &markup(AvatarGroup![Avatar![name = "a"]]);
    for shadow in [Shadow::None, Shadow::Sm, Shadow::Md, Shadow::Lg] {
        all += &markup(Card![
            shadow = shadow,
            bordered = true,
            blurred = true,
            href = "/x",
            CardHeader!["h"],
            CardBody!["b"],
            CardFooter!["f"]
        ]);
    }
    all += &markup(Divider![vertical = true]);
    for w in [Width::Sm, Width::Md, Width::Lg, Width::Xl, Width::Xxl, Width::Full] {
        all += &markup(Container![width = w]);
    }
    for gap in [0, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16] {
        all += &markup(Stack![gap = gap, row = true, wrap = true]);
    }
    for a in [Align::Start, Align::Center, Align::End, Align::Stretch, Align::Baseline] {
        all += &markup(Stack![align = a]);
    }
    for j in [Justify::Start, Justify::Center, Justify::End, Justify::Between, Justify::Around, Justify::Evenly] {
        all += &markup(Stack![justify = j]);
    }
    for cols in 1..=12 {
        all += &markup(Grid![cols = cols]);
    }
    all += &markup(Grid![min_width = "10rem"]);
    all += &markup(AppShell![
        navbar =
            Navbar![brand = "b", menu_toggle = true, bordered = true, end_content = "e", NavbarItem![href = "/", "x"]],
        sidebar = Sidebar![title = "t", SidebarItem![href = "/", icon = "i", "x"]],
        footer = "f",
        "main",
    ]);

    let rendered = classes(&all);
    let css = next_rust_ui::style::UI_CSS_SOURCE;
    let components = &css[css.find("@layer components {").unwrap()..];
    let leading = next_rust_assets::css::class_selectors(components).leading;
    let missing: Vec<&String> = leading.iter().filter(|c| !rendered.contains(*c)).collect();
    assert!(missing.is_empty(), "CSS rules keyed on classes no component renders: {missing:?}");
}
