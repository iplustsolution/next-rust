//! Avatars: an image in a circle, with initials when there is no image.

use next_rust_view::{Attr, Element, Node, View};

use crate::button::common_methods;
use crate::{Color, Extra, Radius, Size, apply, attr, root, styled};

/// A user picture. Without `src`, or when the image fails to load, it shows
/// the initials of `name`.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(Avatar![name = "Ada Lovelace", size = Size::Lg, bordered = true]);
/// assert!(html.contains(r#"<span class="nr-avatar-fallback" aria-hidden="true">AL</span>"#));
/// ```
#[derive(Default)]
pub struct Avatar {
    src: Option<String>,
    name: Option<String>,
    initials: Option<String>,
    size: Size,
    radius: Option<Radius>,
    color: Color,
    bordered: bool,
    unstyled: bool,
    extra: Extra,
}

impl Avatar {
    pub fn new() -> Self {
        Self { color: Color::Default, ..Self::default() }
    }
    common_methods!();

    /// Add an attribute to the outer element, or content shown instead of
    /// the initials (an icon).
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.extra.push(part);
        self
    }
    /// Image URL.
    pub fn src(mut self, v: impl Into<String>) -> Self {
        self.src = Some(v.into());
        self
    }
    /// The person's name: the image's alt text and the source of the initials.
    pub fn name(mut self, v: impl Into<String>) -> Self {
        self.name = Some(v.into());
        self
    }
    /// Text shown without an image (defaults to the initials of `name`).
    pub fn initials(mut self, v: impl Into<String>) -> Self {
        self.initials = Some(v.into());
        self
    }
    pub fn size(mut self, v: Size) -> Self {
        self.size = v;
        self
    }
    /// Rounding (a circle by default).
    pub fn radius(mut self, v: Radius) -> Self {
        self.radius = Some(v);
        self
    }
    /// Color of the ring and of the initials' background.
    pub fn color(mut self, v: Color) -> Self {
        self.color = v;
        self
    }
    /// A ring around the avatar.
    pub fn bordered(mut self, v: bool) -> Self {
        self.bordered = v;
        self
    }
}

/// Initials of a name: "Ada Lovelace" → "AL", "ada" → "A".
fn initials_of(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let first = |w: &str| w.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    match words.as_slice() {
        [] => String::new(),
        [one] => first(one),
        [a, .., b] => first(a) + &first(b),
    }
}

impl View for Avatar {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let size = format!("nr-avatar-{}", self.size.suffix());
        let classes = [
            "nr-avatar",
            &size,
            self.color.class(),
            self.radius.map(Radius::class).unwrap_or(""),
            if self.bordered { "nr-avatar-bordered" } else { "" },
        ];
        let mut el = root("span", &classes, self.unstyled, class);
        match (&self.src, &self.name) {
            (Some(_), _) => {}
            (None, Some(name)) => {
                el.set_attr(attr("role", "img"));
                el.set_attr(attr("aria-label", name.clone()));
            }
            (None, None) => {}
        }
        apply(&mut el, attrs);
        let fallback: Node = if children.is_empty() {
            self.initials.clone().or_else(|| self.name.as_deref().map(initials_of)).unwrap_or_default().into_node()
        } else {
            Node::Fragment(children)
        };
        el.children.push(
            Element::new("span")
                .with(attr("class", "nr-avatar-fallback"))
                .with(attr("aria-hidden", "true"))
                .with(fallback)
                .into_node(),
        );
        if let Some(src) = self.src {
            // `data-nr-ui`: the script hides the image if it fails to load.
            el.set_attr(attr("data-nr-ui", "avatar"));
            el.children.push(
                Element::new_void("img")
                    .with(attr("class", "nr-avatar-img"))
                    .with(attr("src", src))
                    .with(attr("alt", self.name.unwrap_or_default()))
                    .with(attr("loading", "lazy"))
                    .with(attr("decoding", "async"))
                    .into_node(),
            );
        }
        styled(el)
    }
}

/// Values accepted inside `AvatarGroup![..]`.
pub trait AvatarGroupPart {
    fn add_to(self, group: &mut AvatarGroup);
}
impl AvatarGroupPart for Avatar {
    fn add_to(self, group: &mut AvatarGroup) {
        group.avatars.push(self);
    }
}
impl AvatarGroupPart for Vec<Avatar> {
    fn add_to(self, group: &mut AvatarGroup) {
        group.avatars.extend(self);
    }
}
impl AvatarGroupPart for Attr {
    fn add_to(self, group: &mut AvatarGroup) {
        group.extra.push(self);
    }
}

/// Overlapping avatars, with a `+N` count past `max`.
#[derive(Default)]
pub struct AvatarGroup {
    avatars: Vec<Avatar>,
    max: Option<usize>,
    total: Option<usize>,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl AvatarGroup {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();

    /// Add avatars or attributes for the group.
    pub fn with(mut self, part: impl AvatarGroupPart) -> Self {
        part.add_to(&mut self);
        self
    }
    /// Show at most this many avatars.
    pub fn max(mut self, v: usize) -> Self {
        self.max = Some(v);
        self
    }
    /// Total number of people, when more exist than were given.
    pub fn total(mut self, v: usize) -> Self {
        self.total = Some(v);
        self
    }
    pub fn size(mut self, v: Size) -> Self {
        self.size = v;
        self
    }
}

impl View for AvatarGroup {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("div", &["nr-avatar-group"], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        apply(&mut el, attrs);
        let total = self.total.unwrap_or(self.avatars.len()).max(self.avatars.len());
        let shown = self.max.unwrap_or(usize::MAX).min(self.avatars.len());
        for avatar in self.avatars.into_iter().take(shown) {
            el.children.push(avatar.size(self.size).bordered(true).into_node());
        }
        if total > shown {
            let rest = format!("+{}", total - shown);
            el.children.push(
                Avatar::new()
                    .size(self.size)
                    .bordered(true)
                    .initials(rest)
                    .with(attr("aria-label", format!("{} more", total - shown)))
                    .into_node(),
            );
        }
        styled(el)
    }
}
