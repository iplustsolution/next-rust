//! Cards.

use next_rust_view::{Node, View};

use crate::button::{common_methods, with_parts};
use crate::{Extra, Press, Radius, apply, attr, root, styled};

/// Shadow depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shadow {
    None,
    Sm,
    #[default]
    Md,
    Lg,
}

/// A surface grouping related content, with optional header and footer.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let card = Card![CardHeader![h3!["Plan"]], CardBody![p!["Pro"]], CardFooter![Button!["Upgrade"]]];
/// assert!(render_static(card).contains(r#"<div class="nr-card nr-card-shadow-md">"#));
/// ```
#[derive(Default)]
pub struct Card {
    shadow: Shadow,
    radius: Option<Radius>,
    bordered: bool,
    blurred: bool,
    hoverable: bool,
    href: Option<String>,
    press: Option<Press>,
    unstyled: bool,
    extra: Extra,
}

impl Card {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn shadow(mut self, v: Shadow) -> Self {
        self.shadow = v;
        self
    }
    pub fn radius(mut self, v: Radius) -> Self {
        self.radius = Some(v);
        self
    }
    /// A thin border instead of (or with) the shadow.
    pub fn bordered(mut self, v: bool) -> Self {
        self.bordered = v;
        self
    }
    /// Frosted glass: translucent with a background blur.
    pub fn blurred(mut self, v: bool) -> Self {
        self.blurred = v;
        self
    }
    /// Lift slightly on hover.
    pub fn hoverable(mut self, v: bool) -> Self {
        self.hoverable = v;
        self
    }
    /// Make the whole card a link.
    pub fn href(mut self, v: impl Into<String>) -> Self {
        self.href = Some(v.into());
        self
    }
    /// Make the card pressable.
    pub fn on_press(mut self, v: impl Into<Press>) -> Self {
        self.press = Some(v.into());
        self
    }
}

impl View for Card {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let shadow = match self.shadow {
            Shadow::None => "",
            Shadow::Sm => "nr-card-shadow-sm",
            Shadow::Md => "nr-card-shadow-md",
            Shadow::Lg => "nr-card-shadow-lg",
        };
        let pressable = self.href.is_some() || self.press.is_some();
        let classes = [
            "nr-card",
            shadow,
            self.radius.map(Radius::class).unwrap_or(""),
            if self.bordered { "nr-card-bordered" } else { "" },
            if self.blurred { "nr-card-blurred" } else { "" },
            if self.hoverable || pressable { "nr-card-hoverable" } else { "" },
            if pressable { "nr-card-pressable" } else { "" },
        ];
        let tag = if self.href.is_some() { "a" } else { "div" };
        let mut el = root(tag, &classes, self.unstyled, class);
        if let Some(href) = self.href {
            el.set_attr(attr("href", href));
            el.set_attr(attr("data-nr-link", ""));
        }
        if let Some(press) = &self.press {
            el.set_attr(attr("role", "button"));
            el.set_attr(attr("tabindex", "0"));
            apply(&mut el, press.attrs());
        }
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

macro_rules! card_part {
    ($name:ident, $tag:literal, $class:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Default)]
        pub struct $name {
            unstyled: bool,
            extra: Extra,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }
            common_methods!();
            with_parts!();
        }

        impl View for $name {
            fn into_node(self) -> Node {
                let (class, attrs, children) = self.extra.split();
                let mut el = root($tag, &[$class], self.unstyled, class);
                apply(&mut el, attrs);
                el.children = children;
                styled(el)
            }
        }
    };
}

card_part!(CardHeader, "div", "nr-card-header", "The top of a [`Card`].");
card_part!(CardBody, "div", "nr-card-body", "The main content of a [`Card`].");
card_part!(CardFooter, "div", "nr-card-footer", "The bottom of a [`Card`], typically actions.");
