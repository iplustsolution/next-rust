//! Icons used inside the components: Lucide, from `next-rust-icons`.

use next_rust_icons::Icon;
use next_rust_view::{Node, View};

/// An icon sized and colored by the component stylesheet (`.nr-icon`).
pub(crate) fn icon(icon: Icon) -> Node {
    icon.unstyled(true).class("nr-icon").into_node()
}

pub(crate) use next_rust_icons::{
    Calendar as calendar, Check as check, ChevronDown as chevron_down, Eye as eye, EyeOff as eye_off, Menu as menu,
    X as close,
};
