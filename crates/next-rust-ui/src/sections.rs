//! Page sections for marketing sites (hero, features, pricing, testimonials,
//! logos, calls to action, FAQ, footer) and the layout helpers around them.
//! All of it is HTML and CSS: nothing here needs a script except the
//! carousel's optional buttons.

use next_rust_view::{Element, Node, View, raw_html};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::layout::{Align, Width};
use crate::navigation::{Accordion, AccordionItem, AccordionVariant};
use crate::{Avatar, Color, Extra, Rating, Size, apply, attr, clone_node, flag, part, props, root, styled};

fn width_class(width: Width) -> &'static str {
    match width {
        Width::Sm => "nr-container-sm",
        Width::Md => "nr-container-md",
        Width::Lg => "nr-container-lg",
        Width::Xl => "nr-container-xl",
        Width::Xxl => "nr-container-2xl",
        Width::Full => "nr-container-full",
    }
}

/// An eyebrow, a title and a subtitle.
fn heading(
    tag: &'static str,
    prefix: &str,
    eyebrow: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
) -> Vec<Node> {
    let mut out = Vec::new();
    if let Some(eyebrow) = eyebrow {
        out.push(Element::new("p").with(attr("class", format!("{prefix}-eyebrow"))).with(eyebrow).into_node());
    }
    if let Some(title) = title {
        out.push(Element::new(tag).with(attr("class", format!("{prefix}-title"))).with(title).into_node());
    }
    if let Some(subtitle) = subtitle {
        out.push(Element::new("p").with(attr("class", format!("{prefix}-subtitle"))).with(subtitle).into_node());
    }
    out
}

/// The top of a landing page: a big title, a subtitle, actions and
/// optionally media (a screenshot, an illustration) beside or below.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let h = Hero![eyebrow = "New", title = "Ship faster", subtitle = "One binary.", glow = true,
///     actions = Button![href = "/docs", "Get started"]];
/// let html = render_static(h);
/// assert!(html.contains(r#"<h1 class="nr-hero-title">Ship faster</h1>"#) && html.contains("nr-hero-glow"), "{html}");
/// ```
#[derive(Default)]
pub struct Hero {
    eyebrow: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    actions: Option<Node>,
    media: Option<Node>,
    align: Option<Align>,
    split: bool,
    glow: bool,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl Hero {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// A small line above the title.
        eyebrow: text,
        title: text,
        subtitle: text,
        /// Buttons under the text.
        actions: node,
        /// An image, a video, a card: below the text, or beside it with `split`.
        media: node,
        /// `Center` (default) or `Start`.
        align: opt<Align>,
        /// Text on one side, media on the other (from 1024px up).
        split: val<bool>,
        /// A soft glow of the color behind the title.
        glow: val<bool>,
        /// Color of the eyebrow and the glow.
        color: val<Color>,
    }
}

impl View for Hero {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let start = matches!(self.align, Some(Align::Start)) || (self.split && self.align.is_none());
        let classes = [
            "nr-hero",
            self.color.class(),
            if start { "nr-hero-start" } else { "" },
            if self.split { "nr-hero-split" } else { "" },
            if self.glow { "nr-hero-glow" } else { "" },
        ];
        let mut el = root("section", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        let mut text = part("div", "nr-hero-text");
        text.children = heading("h1", "nr-hero", self.eyebrow, self.title, self.subtitle);
        if let Some(actions) = self.actions {
            text.children.push(part("div", "nr-hero-actions").with(actions).into_node());
        }
        text.children.extend(children);
        let mut inner = part("div", "nr-hero-inner").with(text);
        if let Some(media) = self.media {
            inner.children.push(part("div", "nr-hero-media").with(media).into_node());
        }
        el.children.push(inner.into_node());
        styled(el)
    }
}

/// A band of a page with a centered heading and vertical rhythm.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = Section![id = "pricing", eyebrow = "Pricing", title = "Simple plans", muted = true, p!["…"]];
/// let html = render_static(s);
/// assert!(html.contains(r#"<section class="nr-section nr-c-primary nr-section-muted" id="pricing">"#), "{html}");
/// ```
#[derive(Default)]
pub struct Section {
    id: Option<String>,
    eyebrow: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    align: Option<Align>,
    width: Option<Width>,
    muted: bool,
    compact: bool,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl Section {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// An anchor for in-page links.
        id: text,
        eyebrow: text,
        title: text,
        subtitle: text,
        /// `Center` (default) or `Start`, for the heading.
        align: opt<Align>,
        /// Width of the content (`Xl` by default).
        width: opt<Width>,
        /// A tinted background.
        muted: val<bool>,
        /// Less vertical space.
        compact: val<bool>,
        /// Color of the eyebrow.
        color: val<Color>,
    }
}

impl View for Section {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = [
            "nr-section",
            self.color.class(),
            if self.muted { "nr-section-muted" } else { "" },
            if self.compact { "nr-section-compact" } else { "" },
            if matches!(self.align, Some(Align::Start)) { "nr-section-start" } else { "" },
        ];
        let mut el = root("section", &classes, self.unstyled, class);
        if let Some(id) = self.id {
            el.set_attr(attr("id", id));
        }
        apply(&mut el, attrs);
        let mut inner = Element::new("div").with(attr(
            "class",
            format!("nr-section-inner nr-container {}", width_class(self.width.unwrap_or(Width::Xl))),
        ));
        let head = heading("h2", "nr-section", self.eyebrow, self.title, self.subtitle);
        if !head.is_empty() {
            inner.children.push(part("header", "nr-section-head").with(head).into_node());
        }
        inner.children.extend(children);
        el.children.push(inner.into_node());
        styled(el)
    }
}

/// A grid of [`Feature`]s.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let g = FeatureGrid![cols = 3, Feature![title = "Fast", icon = "⚡", "Renders in microseconds."]];
/// let html = render_static(g);
/// assert!(html.contains(r#"class="nr-features nr-grid nr-cols-3""#) && html.contains("nr-feature-icon"), "{html}");
/// ```
#[derive(Default)]
pub struct FeatureGrid {
    cols: Option<u8>,
    unstyled: bool,
    extra: Extra,
}

impl FeatureGrid {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Columns on wide screens, 1 to 4 (3 by default); fewer on small ones.
        cols: opt<u8>,
    }
}

impl View for FeatureGrid {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let cols = format!("nr-cols-{}", self.cols.unwrap_or(3).clamp(1, 4));
        let mut el = root("div", &["nr-features", "nr-grid", &cols], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// One feature: an icon, a title and a short text. With `href` the whole
/// block is a link.
#[derive(Default)]
pub struct Feature {
    icon: Option<Node>,
    title: Option<String>,
    href: Option<String>,
    color: Color,
    card: bool,
    unstyled: bool,
    extra: Extra,
}

impl Feature {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        icon: node,
        title: text,
        href: text,
        /// Color of the icon tile.
        color: val<Color>,
        /// Draw it as a card (border, padding).
        card: val<bool>,
    }
}

impl View for Feature {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let tag = if self.href.is_some() { "a" } else { "div" };
        let mut el = root(
            tag,
            &["nr-feature", self.color.class(), if self.card { "nr-feature-card" } else { "" }],
            self.unstyled,
            class,
        );
        if let Some(href) = self.href {
            el.set_attr(attr("href", href));
            el.set_attr(attr("data-nr-link", ""));
        }
        apply(&mut el, attrs);
        if let Some(glyph) = self.icon {
            el.children.push(part("span", "nr-feature-icon").with(attr("aria-hidden", "true")).with(glyph).into_node());
        }
        if let Some(title) = self.title {
            el.children.push(part("h3", "nr-feature-title").with(title).into_node());
        }
        if !children.is_empty() {
            el.children.push(part("div", "nr-feature-text").with(children).into_node());
        }
        styled(el)
    }
}

/// A row of [`PricingCard`]s (stacked on small screens).
#[derive(Default)]
pub struct PricingTable {
    unstyled: bool,
    extra: Extra,
}

impl PricingTable {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
}

impl View for PricingTable {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-pricing"], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A plan: name, price, features and a call to action.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let p = PricingCard![name = "Pro", price = "$19", period = "/month", featured = true, badge = "Popular",
///     cta = Button![href = "/signup", full_width = true, "Choose Pro"],
///     PricingFeature!["Unlimited projects"], PricingFeature![included = false, "SSO"]];
/// let html = render_static(p);
/// assert!(html.contains("nr-price-card-featured") && html.contains("$19") && html.contains("Not included"), "{html}");
/// ```
#[derive(Default)]
pub struct PricingCard {
    name: Option<String>,
    price: Option<String>,
    period: Option<String>,
    description: Option<String>,
    badge: Option<String>,
    featured: bool,
    cta: Option<Node>,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl PricingCard {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        name: text,
        /// "$19", "Free", "Custom".
        price: text,
        /// "/month".
        period: text,
        description: text,
        /// A small label in the corner ("Popular").
        badge: text,
        /// Highlight this plan.
        featured: val<bool>,
        /// The button.
        cta: node,
        /// Color of the highlight and the check marks.
        color: val<Color>,
    }
}

impl View for PricingCard {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = ["nr-price-card", self.color.class(), if self.featured { "nr-price-card-featured" } else { "" }];
        let mut el = root("article", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        let mut head = part("header", "nr-price-head");
        if let Some(name) = self.name {
            head.children.push(part("h3", "nr-price-name").with(name).into_node());
        }
        if let Some(badge) = self.badge {
            head.children.push(part("span", "nr-price-badge").with(badge).into_node());
        }
        el.children.push(head.into_node());
        if let Some(price) = self.price {
            let mut line = part("p", "nr-price-amount").with(part("span", "nr-price-value").with(price));
            if let Some(period) = self.period {
                line.children.push(part("span", "nr-price-period").with(period).into_node());
            }
            el.children.push(line.into_node());
        }
        if let Some(description) = self.description {
            el.children.push(part("p", "nr-price-description").with(description).into_node());
        }
        if let Some(cta) = self.cta {
            el.children.push(part("div", "nr-price-cta").with(cta).into_node());
        }
        if !children.is_empty() {
            el.children.push(part("ul", "nr-price-features").with(children).into_node());
        }
        styled(el)
    }
}

/// A line of a [`PricingCard`]: included (a check) or not (a dash).
pub struct PricingFeature {
    included: bool,
    unstyled: bool,
    extra: Extra,
}

impl Default for PricingFeature {
    fn default() -> Self {
        Self { included: true, unstyled: false, extra: Extra::default() }
    }
}

impl PricingFeature {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        included: val<bool>,
    }
}

impl View for PricingFeature {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = ["nr-price-feature", if self.included { "" } else { "nr-price-feature-off" }];
        let mut el = root("li", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        el.children.push(icon(if self.included { icons::check() } else { icons::minus() }));
        let mut text = Element::new("span");
        if !self.included {
            text.children.push(part("span", "nr-sr").with("Not included: ").into_node());
        }
        text.children.extend(children);
        el.children.push(text.into_node());
        styled(el)
    }
}

/// A quote from a customer.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let t = Testimonial![name = "Ada Lovelace", role = "CTO, Analytical", rating = 5.0, "It just works."];
/// let html = render_static(t);
/// assert!(html.contains("<blockquote") && html.contains("<figcaption") && html.contains("Ada Lovelace"), "{html}");
/// ```
#[derive(Default)]
pub struct Testimonial {
    name: Option<String>,
    role: Option<String>,
    avatar: Option<String>,
    rating: Option<f64>,
    logo: Option<Node>,
    bordered: bool,
    unstyled: bool,
    extra: Extra,
}

impl Testimonial {
    pub fn new() -> Self {
        Self { bordered: true, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// Who said it.
        name: text,
        /// Their title and company.
        role: text,
        /// URL of their photo (initials when not given).
        avatar: text,
        /// Stars above the quote.
        rating: opt<f64>,
        /// A company logo at the end of the caption.
        logo: node,
        /// A card around the quote (on by default).
        bordered: val<bool>,
    }
}

impl View for Testimonial {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root(
            "figure",
            &["nr-testimonial", if self.bordered { "nr-testimonial-card" } else { "" }],
            self.unstyled,
            class,
        );
        apply(&mut el, attrs);
        if let Some(rating) = self.rating {
            el.children.push(Rating::new().value(rating).readonly(true).size(Size::Sm).into_node());
        }
        el.children.push(part("blockquote", "nr-testimonial-quote").with(children).into_node());
        if self.name.is_some() || self.role.is_some() {
            let mut caption = part("figcaption", "nr-testimonial-author");
            if let Some(name) = &self.name {
                let mut avatar = Avatar::new().name(name.clone());
                if let Some(src) = self.avatar {
                    avatar = avatar.src(src);
                }
                caption.children.push(avatar.into_node());
            }
            let mut who = part("span", "nr-testimonial-who");
            if let Some(name) = self.name {
                who.children.push(part("strong", "nr-testimonial-name").with(name).into_node());
            }
            if let Some(role) = self.role {
                who.children.push(part("span", "nr-testimonial-role").with(role).into_node());
            }
            caption.children.push(who.into_node());
            if let Some(logo) = self.logo {
                caption.children.push(part("span", "nr-testimonial-logo").with(logo).into_node());
            }
            el.children.push(caption.into_node());
        }
        styled(el)
    }
}

/// A row of customer or partner logos: muted until hovered.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let l = LogoCloud![title = "Trusted by teams at", img![src("/acme.svg"), alt("Acme")], span!["Globex"]];
/// assert!(render_static(l).contains(r#"<li class="nr-logos-item"><img src="/acme.svg" alt="Acme">"#));
/// ```
#[derive(Default)]
pub struct LogoCloud {
    title: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl LogoCloud {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// A muted line above the logos.
        title: text,
    }
}

impl View for LogoCloud {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-logos"], self.unstyled, class);
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.children.push(part("p", "nr-logos-title").with(title).into_node());
        }
        let mut list = part("ul", "nr-logos-list");
        for child in children {
            if matches!(child, Node::Empty | Node::Style(_)) {
                list.children.push(child);
            } else {
                list.children.push(part("li", "nr-logos-item").with(child).into_node());
            }
        }
        el.children.push(list.into_node());
        styled(el)
    }
}

/// A call-to-action banner: a title, a line of text and buttons.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = Cta![title = "Ready to ship?", description = "Start free.", Button![href = "/signup", "Get started"]];
/// assert!(render_static(c).contains(r#"<section class="nr-cta nr-cta-solid nr-c-primary">"#));
/// ```
#[derive(Default)]
pub struct Cta {
    title: Option<String>,
    description: Option<String>,
    color: Color,
    soft: bool,
    split: bool,
    unstyled: bool,
    extra: Extra,
}

impl Cta {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
        description: text,
        color: val<Color>,
        /// A soft tint instead of the solid color.
        soft: val<bool>,
        /// Text on the left, buttons on the right (from 768px up); centered otherwise.
        split: val<bool>,
    }
}

impl View for Cta {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = [
            "nr-cta",
            if self.soft { "nr-cta-soft" } else { "nr-cta-solid" },
            self.color.class(),
            if self.split { "nr-cta-split" } else { "" },
        ];
        let mut el = root("section", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        let mut text = part("div", "nr-cta-text");
        if let Some(title) = self.title {
            text.children.push(part("h2", "nr-cta-title").with(title).into_node());
        }
        if let Some(description) = self.description {
            text.children.push(part("p", "nr-cta-description").with(description).into_node());
        }
        el.children.push(text.into_node());
        if !children.is_empty() {
            el.children.push(part("div", "nr-cta-actions").with(children).into_node());
        }
        styled(el)
    }
}

/// Questions and answers: an [`Accordion`](struct@crate::Accordion) preset
/// that can also emit `FAQPage` structured data for search engines.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let f = Faq![structured_data = true, [("Is it free?", "Yes, MIT or Apache-2.0.")]];
/// let html = render_static(f);
/// assert!(html.contains("<details") && html.contains(r#""@type":"FAQPage""#), "{html}");
/// ```
#[derive(Default)]
pub struct Faq {
    items: Vec<(String, String)>,
    variant: Option<AccordionVariant>,
    structured_data: bool,
    extra: Extra,
}

/// Values accepted inside `Faq![..]`.
pub trait FaqPart {
    fn add_to(self, faq: &mut Faq);
}
impl FaqPart for next_rust_view::Attr {
    fn add_to(self, faq: &mut Faq) {
        faq.extra.push(self);
    }
}
/// `("Question", "Answer")`.
impl<Q: Into<String>, A: Into<String>> FaqPart for (Q, A) {
    fn add_to(self, faq: &mut Faq) {
        faq.items.push((self.0.into(), self.1.into()));
    }
}
impl<Q: Into<String>, A: Into<String>, const N: usize> FaqPart for [(Q, A); N] {
    fn add_to(self, faq: &mut Faq) {
        for item in self {
            item.add_to(faq);
        }
    }
}
impl<Q: Into<String>, A: Into<String>> FaqPart for Vec<(Q, A)> {
    fn add_to(self, faq: &mut Faq) {
        for item in self {
            item.add_to(faq);
        }
    }
}

impl Faq {
    pub fn new() -> Self {
        Self::default()
    }
    props! {
        /// `Splitted` by default.
        variant: opt<AccordionVariant>,
        /// Also emit a `FAQPage` JSON-LD script.
        structured_data: val<bool>,
    }
    /// Extra classes for the accordion.
    pub fn class(mut self, v: impl Into<String>) -> Self {
        self.extra.push(next_rust_view::attrs::class(v.into()));
        self
    }
    /// Add questions (`("Question", "Answer")`, arrays or a `Vec` of them) or attributes.
    pub fn with(mut self, part: impl FaqPart) -> Self {
        part.add_to(&mut self);
        self
    }
}

impl View for Faq {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut out = Vec::new();
        if self.structured_data && !self.items.is_empty() {
            let entities: Vec<serde_json::Value> = self
                .items
                .iter()
                .map(|(q, a)| {
                    serde_json::json!({
                        "@type": "Question",
                        "name": q,
                        "acceptedAnswer": { "@type": "Answer", "text": a },
                    })
                })
                .collect();
            let json =
                serde_json::json!({ "@context": "https://schema.org", "@type": "FAQPage", "mainEntity": entities });
            // `<` cannot end the script element early.
            let json = json.to_string().replace('<', "\\u003c");
            out.push(raw_html(format!(r#"<script type="application/ld+json">{json}</script>"#)));
        }
        let mut accordion = Accordion::new().variant(self.variant.unwrap_or(AccordionVariant::Splitted));
        if let Some(class) = class {
            accordion = accordion.with(class);
        }
        for a in attrs {
            accordion = accordion.with(a);
        }
        for (question, answer) in self.items {
            accordion = accordion.with(AccordionItem::new().title(question).with(Element::new("p").with(answer)));
        }
        out.push(accordion.into_node());
        Node::Fragment(out)
    }
}

/// Events in order, along a line.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let t = Timeline![TimelineItem![title = "Ordered", time = "09:12", color = Color::Success, "Payment received."]];
/// let html = render_static(t);
/// assert!(html.contains(r#"<ol class="nr-timeline">"#) && html.contains("<time"), "{html}");
/// ```
#[derive(Default)]
pub struct Timeline {
    unstyled: bool,
    extra: Extra,
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
}

impl View for Timeline {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("ol", &["nr-timeline"], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// An event of a [`Timeline`].
#[derive(Default)]
pub struct TimelineItem {
    title: Option<String>,
    time: Option<String>,
    datetime: Option<String>,
    icon: Option<Node>,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl TimelineItem {
    pub fn new() -> Self {
        Self { color: Color::Default, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
        /// The shown time ("2 hours ago", "12 Sep").
        time: text,
        /// The machine-readable time (`datetime` of the `<time>`).
        datetime: text,
        /// An icon in the marker instead of a dot.
        icon: node,
        color: val<Color>,
    }
}

impl View for TimelineItem {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("li", &["nr-timeline-item", self.color.class()], self.unstyled, class);
        apply(&mut el, attrs);
        let mut marker = part("span", "nr-timeline-marker").with(attr("aria-hidden", "true"));
        if let Some(glyph) = self.icon {
            marker.set_attr(attr("data-icon", ""));
            marker.children.push(glyph);
        }
        el.children.push(marker.into_node());
        let mut body = part("div", "nr-timeline-body");
        let mut head = part("div", "nr-timeline-head");
        if let Some(title) = self.title {
            head.children.push(part("strong", "nr-timeline-title").with(title).into_node());
        }
        if let Some(time) = self.time {
            let mut t = part("time", "nr-timeline-time");
            if let Some(datetime) = self.datetime {
                t.set_attr(attr("datetime", datetime));
            }
            head.children.push(t.with(time).into_node());
        }
        body.children.push(head.into_node());
        if !children.is_empty() {
            body.children.push(part("div", "nr-timeline-text").with(children).into_node());
        }
        el.children.push(body.into_node());
        styled(el)
    }
}

/// Slides you swipe or scroll through: CSS scroll snapping, so touch, the
/// trackpad and the keyboard (focus it, then the arrow keys) work with no
/// script. The component script adds previous/next buttons.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = Carousel![label = "Screenshots", per_view = 2, CarouselItem![img![src("/1.png"), alt("Home")]], CarouselItem!["2"]];
/// let html = render_static(c);
/// assert!(html.contains(r#"aria-roledescription="carousel""#) && html.contains(r#"aria-label="1 of 2""#), "{html}");
/// ```
#[derive(Default)]
pub struct Carousel {
    label: Option<String>,
    per_view: Option<u8>,
    gap: Option<u8>,
    controls: Option<bool>,
    peek: bool,
    unstyled: bool,
    extra: Extra,
}

impl Carousel {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Accessible name ("Customer stories").
        label: text,
        /// Slides visible at once on wide screens, 1 to 4 (1 by default); one at a time on phones.
        per_view: opt<u8>,
        /// Space between slides in quarters of a rem (4 by default).
        gap: opt<u8>,
        /// Previous/next buttons (on by default; they need the script).
        controls: opt<bool>,
        /// Let the next slide peek in from the edge.
        peek: val<bool>,
    }
}

impl View for Carousel {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let per = format!("nr-carousel-{}", self.per_view.unwrap_or(1).clamp(1, 4));
        let mut el =
            root("div", &["nr-carousel", &per, if self.peek { "nr-carousel-peek" } else { "" }], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        el.set_attr(attr("aria-roledescription", "carousel"));
        el.set_attr(attr("aria-label", self.label.unwrap_or_else(|| "Carousel".to_owned())));
        let controls = self.controls.unwrap_or(true);
        if controls {
            el.set_attr(attr("data-nr-ui", "carousel"));
        }
        if let Some(gap) = self.gap {
            el.set_attr(attr("style", format!("--nr-carousel-gap:{}rem", f64::from(gap) * 0.25)));
        }
        apply(&mut el, attrs);
        let mut slides: Vec<Node> = children;
        let total = slides.iter_mut().map(first_is_slide).filter(|slide| *slide).count();
        let mut index = 0;
        for slide in &mut slides {
            if let Some(item) = crate::first_element(std::slice::from_mut(slide))
                && item.attrs.iter().any(|a| a.name == "aria-roledescription")
            {
                index += 1;
                item.set_attr(attr("aria-label", format!("{index} of {total}")));
            }
        }
        let mut viewport = part("div", "nr-carousel-viewport").with(attr("tabindex", "0"));
        viewport.children = slides;
        el.children.push(viewport.into_node());
        if controls {
            for (class, label, dir, glyph) in [
                ("nr-carousel-btn nr-carousel-prev", "Previous slide", "-1", icons::chevron_left()),
                ("nr-carousel-btn nr-carousel-next", "Next slide", "1", icons::chevron_right()),
            ] {
                el.children.push(
                    Element::new("button")
                        .with(attr("type", "button"))
                        .with(attr("class", class))
                        .with(attr("aria-label", label))
                        .with(attr("data-nr-slide", dir))
                        .with(icon(glyph))
                        .into_node(),
                );
            }
        }
        styled(el)
    }
}

fn first_is_slide(node: &mut Node) -> bool {
    crate::first_element(std::slice::from_mut(node))
        .is_some_and(|el| el.attrs.iter().any(|a| a.name == "aria-roledescription"))
}

/// A slide of a [`Carousel`].
#[derive(Default)]
pub struct CarouselItem {
    unstyled: bool,
    extra: Extra,
}

impl CarouselItem {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
}

impl View for CarouselItem {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-carousel-item"], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        el.set_attr(attr("aria-roledescription", "slide"));
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// Content that scrolls sideways forever (logos, short quotes). CSS only;
/// it pauses on hover and focus, and with reduced motion it stands still
/// and scrolls by hand instead.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let m = Marquee![speed = 30, span!["Acme"], span!["Globex"]];
/// let html = render_static(m);
/// assert_eq!(html.matches("Acme").count(), 2, "the content, and a hidden copy for the loop");
/// assert!(html.contains(r#"aria-hidden="true""#) && html.contains("--nr-marquee-d:30s"));
/// ```
#[derive(Default)]
pub struct Marquee {
    speed: Option<u32>,
    reverse: bool,
    fade: Option<bool>,
    gap: Option<u8>,
    unstyled: bool,
    extra: Extra,
}

impl Marquee {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Seconds for one full loop (40 by default).
        speed: opt<u32>,
        /// Scroll the other way.
        reverse: val<bool>,
        /// Fade the two edges out (on by default).
        fade: opt<bool>,
        /// Space between items in quarters of a rem (12 by default).
        gap: opt<u8>,
    }
}

impl View for Marquee {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = [
            "nr-marquee",
            if self.reverse { "nr-marquee-reverse" } else { "" },
            if self.fade.unwrap_or(true) { "nr-marquee-fade" } else { "" },
        ];
        let mut el = root("div", &classes, self.unstyled, class);
        let mut style = format!("--nr-marquee-d:{}s", self.speed.unwrap_or(40).max(1));
        if let Some(gap) = self.gap {
            style.push_str(&format!(";--nr-marquee-gap:{}rem", f64::from(gap) * 0.25));
        }
        el.set_attr(attr("style", style));
        apply(&mut el, attrs);
        let copy: Vec<Node> = children.iter().map(clone_node).collect();
        el.children.push(part("div", "nr-marquee-track").with(children).into_node());
        el.children.push(
            part("div", "nr-marquee-track")
                .with(attr("aria-hidden", "true"))
                .with(flag("inert", true))
                .with(copy)
                .into_node(),
        );
        styled(el)
    }
}

/// A grid of tiles of different sizes. One column on phones.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let b = Bento![cols = 3, BentoItem![col_span = 2, row_span = 2, title = "Fast", "…"], BentoItem![title = "Small"]];
/// let html = render_static(b);
/// assert!(html.contains("nr-bento-3") && html.contains("nr-bento-c2 nr-bento-r2"), "{html}");
/// ```
#[derive(Default)]
pub struct Bento {
    cols: Option<u8>,
    unstyled: bool,
    extra: Extra,
}

impl Bento {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Columns on wide screens, 2 to 4 (3 by default).
        cols: opt<u8>,
    }
}

impl View for Bento {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let cols = format!("nr-bento-{}", self.cols.unwrap_or(3).clamp(2, 4));
        let mut el = root("div", &["nr-bento", &cols], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A tile of a [`Bento`] grid.
#[derive(Default)]
pub struct BentoItem {
    col_span: Option<u8>,
    row_span: Option<u8>,
    title: Option<String>,
    description: Option<String>,
    href: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl BentoItem {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Columns the tile covers, 1 to 4.
        col_span: opt<u8>,
        /// Rows the tile covers, 1 to 3.
        row_span: opt<u8>,
        title: text,
        description: text,
        /// Make the tile a link.
        href: text,
    }
}

impl View for BentoItem {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let col = self.col_span.map(|n| format!("nr-bento-c{}", n.clamp(1, 4))).unwrap_or_default();
        let row = self.row_span.map(|n| format!("nr-bento-r{}", n.clamp(1, 3))).unwrap_or_default();
        let tag = if self.href.is_some() { "a" } else { "div" };
        let mut el = root(tag, &["nr-bento-item", &col, &row], self.unstyled, class);
        if let Some(href) = self.href {
            el.set_attr(attr("href", href));
            el.set_attr(attr("data-nr-link", ""));
        }
        apply(&mut el, attrs);
        if self.title.is_some() || self.description.is_some() {
            let mut head = part("div", "nr-bento-head");
            if let Some(title) = self.title {
                head.children.push(part("h3", "nr-bento-title").with(title).into_node());
            }
            if let Some(description) = self.description {
                head.children.push(part("p", "nr-bento-description").with(description).into_node());
            }
            el.children.push(head.into_node());
        }
        if !children.is_empty() {
            el.children.push(part("div", "nr-bento-content").with(children).into_node());
        }
        styled(el)
    }
}

/// A site footer: a brand block, columns of links and a bottom line.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let f = Footer![brand = strong!["Acme"], description = "Tools for teams.", bottom = "© 2026 Acme",
///     FooterColumn![title = "Product", FooterLink![href = "/pricing", "Pricing"]]];
/// let html = render_static(f);
/// assert!(html.contains(r#"<footer class="nr-footer">"#) && html.contains(r#"<nav class="nr-footer-column" aria-label="Product">"#), "{html}");
/// ```
#[derive(Default)]
pub struct Footer {
    brand: Option<Node>,
    description: Option<String>,
    social: Option<Node>,
    bottom: Option<Node>,
    bottom_end: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl Footer {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Logo and name.
        brand: node,
        /// A line under the brand.
        description: text,
        /// Social links under the description.
        social: node,
        /// The bottom line: copyright.
        bottom: node,
        /// The end of the bottom line: legal links, a theme switch.
        bottom_end: node,
    }
}

impl View for Footer {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("footer", &["nr-footer"], self.unstyled, class);
        apply(&mut el, attrs);
        let mut top = part("div", "nr-footer-top");
        if self.brand.is_some() || self.description.is_some() || self.social.is_some() {
            let mut about = part("div", "nr-footer-about");
            if let Some(brand) = self.brand {
                about.children.push(part("div", "nr-footer-brand").with(brand).into_node());
            }
            if let Some(description) = self.description {
                about.children.push(part("p", "nr-footer-description").with(description).into_node());
            }
            if let Some(social) = self.social {
                about.children.push(part("div", "nr-footer-social").with(social).into_node());
            }
            top.children.push(about.into_node());
        }
        if !children.is_empty() {
            top.children.push(part("div", "nr-footer-columns").with(children).into_node());
        }
        if !top.children.is_empty() {
            el.children.push(top.into_node());
        }
        if self.bottom.is_some() || self.bottom_end.is_some() {
            let mut bottom = part("div", "nr-footer-bottom");
            if let Some(b) = self.bottom {
                bottom.children.push(part("div", "nr-footer-legal").with(b).into_node());
            }
            if let Some(b) = self.bottom_end {
                bottom.children.push(part("div", "nr-footer-end").with(b).into_node());
            }
            el.children.push(bottom.into_node());
        }
        styled(el)
    }
}

/// A titled list of [`FooterLink`]s.
#[derive(Default)]
pub struct FooterColumn {
    title: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl FooterColumn {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
    }
}

impl View for FooterColumn {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("nav", &["nr-footer-column"], self.unstyled, class);
        if let Some(title) = &self.title {
            el.set_attr(attr("aria-label", title.clone()));
        }
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.children.push(part("h3", "nr-footer-heading").with(title).into_node());
        }
        el.children.push(part("ul", "nr-footer-links").with(children).into_node());
        styled(el)
    }
}

/// A link of a [`FooterColumn`].
#[derive(Default)]
pub struct FooterLink {
    href: Option<String>,
    external: bool,
    unstyled: bool,
    extra: Extra,
}

impl FooterLink {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        href: text,
        /// Open in a new tab (`rel="noopener noreferrer"`).
        external: val<bool>,
    }
}

impl View for FooterLink {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut a = root("a", &["nr-footer-link"], self.unstyled, class);
        a.set_attr(attr("href", self.href.unwrap_or_else(|| "#".to_owned())));
        if self.external {
            a.set_attr(attr("target", "_blank"));
            a.set_attr(attr("rel", "noopener noreferrer"));
        } else {
            a.set_attr(attr("data-nr-link", ""));
        }
        apply(&mut a, attrs);
        a.children = children;
        Node::Fragment(vec![Node::Style(&crate::UI_CSS), Element::new("li").with(a).into_node()])
    }
}

/// Page links for a list the server paginates: every page is a plain link.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let p = Pagination![page = 5, total = 20, href = "/posts?page={page}"];
/// let html = render_static(p);
/// assert!(html.contains(r#"<a class="nr-page nr-page-current" href="/posts?page=5" data-nr-link="" aria-current="page""#), "{html}");
/// assert!(html.contains(r#"rel="prev""#) && html.contains(r#"href="/posts?page=20""#) && html.contains("…"));
/// ```
#[derive(Default)]
pub struct Pagination {
    page: Option<u32>,
    total: Option<u32>,
    href: Option<String>,
    siblings: Option<u32>,
    label: Option<String>,
    compact: bool,
    color: Color,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl Pagination {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// The current page, from 1.
        page: opt<u32>,
        /// Number of pages.
        total: opt<u32>,
        /// URL of a page, with `{page}` where the number goes: `"/posts?page={page}"`.
        href: text,
        /// Pages shown on each side of the current one (1 by default).
        siblings: opt<u32>,
        /// Accessible name ("Pagination" by default).
        label: text,
        /// Only previous and next, with "Page 3 of 20".
        compact: val<bool>,
        color: val<Color>,
        size: val<Size>,
    }
}

impl View for Pagination {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let total = self.total.unwrap_or(1).max(1);
        let page = self.page.unwrap_or(1).clamp(1, total);
        let template = self.href.unwrap_or_else(|| "?page={page}".to_owned());
        let url = |n: u32| {
            if template.contains("{page}") {
                template.replace("{page}", &n.to_string())
            } else {
                format!("{template}{}page={n}", if template.contains('?') { '&' } else { '?' })
            }
        };
        let size = format!("nr-pagination-{}", self.size.suffix());
        let mut el = root("nav", &["nr-pagination", self.color.class(), &size], self.unstyled, class);
        el.set_attr(attr("aria-label", self.label.unwrap_or_else(|| "Pagination".to_owned())));
        apply(&mut el, attrs);
        let mut list = part("ul", "nr-pagination-list");
        let step = |target: Option<u32>, rel: &'static str, label: &'static str, glyph: next_rust_icons::Icon| {
            let mut a = match target {
                Some(n) => {
                    Element::new("a").with(attr("href", url(n))).with(attr("data-nr-link", "")).with(attr("rel", rel))
                }
                None => Element::new("span").with(attr("aria-disabled", "true")),
            };
            a.set_attr(attr("class", "nr-page nr-page-step"));
            a.set_attr(attr("aria-label", label));
            Element::new("li").with(a.with(icon(glyph))).into_node()
        };
        list.children.push(step((page > 1).then(|| page - 1), "prev", "Previous page", icons::chevron_left()));
        if self.compact {
            list.children.push(
                Element::new("li")
                    .with(
                        part("span", "nr-page-status")
                            .with(attr("aria-current", "page"))
                            .with(format!("Page {page} of {total}")),
                    )
                    .into_node(),
            );
        } else {
            let siblings = self.siblings.unwrap_or(1);
            let mut last = 0;
            for n in 1..=total {
                let near = n + siblings >= page && n <= page + siblings;
                if !(n == 1 || n == total || near) {
                    continue;
                }
                if n > last + 1 {
                    // A single hidden page shows as itself rather than as an ellipsis.
                    if n == last + 2 {
                        list.children.push(page_link(n - 1, false, url(n - 1)));
                    } else {
                        list.children.push(
                            Element::new("li")
                                .with(part("span", "nr-page nr-page-gap").with(attr("aria-hidden", "true")).with("…"))
                                .into_node(),
                        );
                    }
                }
                list.children.push(page_link(n, n == page, url(n)));
                last = n;
            }
        }
        list.children.push(step((page < total).then(|| page + 1), "next", "Next page", icons::chevron_right()));
        el.children.push(list.into_node());
        styled(el)
    }
}

fn page_link(n: u32, current: bool, href: String) -> Node {
    let mut a = Element::new("a")
        .with(attr("class", if current { "nr-page nr-page-current" } else { "nr-page" }))
        .with(attr("href", href))
        .with(attr("data-nr-link", ""));
    if current {
        a.set_attr(attr("aria-current", "page"));
    } else {
        a.set_attr(attr("aria-label", format!("Page {n}")));
    }
    Element::new("li").with(a.with(n.to_string())).into_node()
}

/// A box that keeps a width-to-height ratio; its child (an image, a video,
/// a map) fills it.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let a = AspectRatio![ratio = "16 / 9", img![src("/cover.jpg"), alt("")]];
/// assert!(render_static(a).contains(r#"<div class="nr-aspect" style="aspect-ratio:16 / 9">"#));
/// ```
#[derive(Default)]
pub struct AspectRatio {
    ratio: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl AspectRatio {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// `"16 / 9"` (default), `"1"`, `"4 / 3"`.
        ratio: text,
    }
}

/// Keep digits, spaces, dots and a slash: a ratio cannot carry other CSS.
fn safe_ratio(ratio: Option<String>) -> String {
    let r: String =
        ratio.unwrap_or_default().chars().filter(|c| c.is_ascii_digit() || matches!(c, ' ' | '.' | '/')).collect();
    if r.trim().is_empty() { "16 / 9".to_owned() } else { r.trim().to_owned() }
}

impl View for AspectRatio {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-aspect"], self.unstyled, class);
        el.set_attr(attr("style", format!("aspect-ratio:{}", safe_ratio(self.ratio))));
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A CSS length made of safe characters (`"20rem"`, `"50vh"`, `"calc(100vh - 4rem)"`).
fn safe_length(v: &str) -> String {
    v.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '%' | '(' | ')' | '-' | '+' | '*' | '/'))
        .collect()
}

/// A scrolling box with thin scrollbars. It can be focused, so the keyboard scrolls it.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = ScrollArea![max_height = "12rem", label = "Changelog", p!["…"]];
/// assert!(render_static(s).contains(r#"tabindex="0" role="region" aria-label="Changelog" style="max-height:12rem""#));
/// ```
#[derive(Default)]
pub struct ScrollArea {
    max_height: Option<String>,
    horizontal: bool,
    label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl ScrollArea {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// A CSS length: `"20rem"`, `"50vh"`.
        max_height: text,
        /// Scroll sideways instead.
        horizontal: val<bool>,
        /// Accessible name of the region.
        label: text,
    }
}

impl View for ScrollArea {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el =
            root("div", &["nr-scroll", if self.horizontal { "nr-scroll-x" } else { "" }], self.unstyled, class);
        el.set_attr(attr("tabindex", "0"));
        el.set_attr(attr("role", "region"));
        el.set_attr(attr("aria-label", self.label.unwrap_or_else(|| "Scrollable content".to_owned())));
        if let Some(h) = self.max_height {
            el.set_attr(attr("style", format!("max-height:{}", safe_length(&h))));
        }
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// Empty space: a fixed size, or whatever is left in a `Stack!` when no size is given.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// assert!(render_static(Spacer![]).contains(r#"class="nr-spacer""#));
/// assert!(render_static(Spacer![size = 8]).contains("--nr-space:2rem"));
/// ```
#[derive(Default)]
pub struct Spacer {
    size: Option<u8>,
    unstyled: bool,
    extra: Extra,
}

impl Spacer {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// In quarters of a rem, both ways.
        size: opt<u8>,
    }
}

impl View for Spacer {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("div", &["nr-spacer"], self.unstyled, class);
        el.set_attr(attr("aria-hidden", "true"));
        if let Some(size) = self.size {
            el.set_attr(attr("style", format!("--nr-space:{}rem", f64::from(size) * 0.25)));
        }
        apply(&mut el, attrs);
        styled(el)
    }
}

/// A screen width where layouts change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Breakpoint {
    /// 640px.
    Sm,
    /// 768px.
    #[default]
    Md,
    /// 1024px.
    Lg,
    /// 1280px.
    Xl,
}

impl Breakpoint {
    fn suffix(self) -> &'static str {
        match self {
            Breakpoint::Sm => "sm",
            Breakpoint::Md => "md",
            Breakpoint::Lg => "lg",
            Breakpoint::Xl => "xl",
        }
    }
}

macro_rules! responsive {
    ($name:ident, $prefix:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Default)]
        pub struct $name {
            above: Breakpoint,
            unstyled: bool,
            extra: Extra,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }
            common_methods!();
            with_parts!();
            props! {
                /// The breakpoint (`Md`, 768px, by default).
                above: val<Breakpoint>,
            }
        }

        impl View for $name {
            fn into_node(self) -> Node {
                let (class, attrs, children) = self.extra.split();
                let name = format!(concat!($prefix, "{}"), self.above.suffix());
                let mut el = root("div", &[&name], self.unstyled, class);
                apply(&mut el, attrs);
                el.children = children;
                styled(el)
            }
        }
    };
}

responsive!(
    Show,
    "nr-show-",
    "Children shown only from a breakpoint up: `Show![above = Breakpoint::Md, ..]`. The wrapper has no box of its own."
);
responsive!(
    Hide,
    "nr-hide-",
    "Children hidden from a breakpoint up (shown below it): `Hide![above = Breakpoint::Md, ..]`."
);

/// Keeps its children at the top of the screen while the page scrolls: a
/// sticky header in one line.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = Sticky![blurred = true, bordered = true, Navbar![brand = "Acme"]];
/// assert!(render_static(s).contains(r#"<div class="nr-sticky nr-sticky-blur nr-sticky-border">"#));
/// ```
#[derive(Default)]
pub struct Sticky {
    top: Option<String>,
    blurred: bool,
    bordered: bool,
    unstyled: bool,
    extra: Extra,
}

impl Sticky {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Distance from the top, a CSS length (`"0"` by default).
        top: text,
        /// A translucent, blurred background.
        blurred: val<bool>,
        /// A line below.
        bordered: val<bool>,
    }
}

impl View for Sticky {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = [
            "nr-sticky",
            if self.blurred { "nr-sticky-blur" } else { "" },
            if self.bordered { "nr-sticky-border" } else { "" },
        ];
        let mut el = root("div", &classes, self.unstyled, class);
        if let Some(top) = self.top {
            el.set_attr(attr("style", format!("--nr-sticky-top:{}", safe_length(&top))));
        }
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}
