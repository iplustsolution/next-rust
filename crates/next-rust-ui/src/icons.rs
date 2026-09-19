//! Icons used inside the components: Lucide, from `next-rust-icons`.

use next_rust_icons::Icon;
use next_rust_view::{Node, View};

/// An icon sized and colored by the component stylesheet (`.nr-icon`).
pub(crate) fn icon(icon: Icon) -> Node {
    icon.class("nr-icon").into_node()
}

pub(crate) use next_rust_icons::{
    Calendar as calendar, Check as check, ChevronDown as chevron_down, ChevronRight as chevron_right,
    CircleCheck as circle_check, CircleX as circle_x, Eye as eye, EyeOff as eye_off, Inbox as inbox, Info as info,
    Menu as menu, Minus as minus, TrendingDown as trending_down, TrendingUp as trending_up,
    TriangleAlert as triangle_alert, X as close,
};
