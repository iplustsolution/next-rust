//! Icons used inside the components: Lucide, from `next-rust-icons`.

use next_rust_icons::Icon;
use next_rust_view::{Node, View};

/// An icon sized and colored by the component stylesheet (`.nr-icon`).
pub(crate) fn icon(icon: Icon) -> Node {
    icon.class("nr-icon").into_node()
}

pub(crate) use next_rust_icons::{
    ArrowDown as arrow_down, ArrowRight as arrow_right, ArrowUp as arrow_up, Calendar as calendar, Check as check,
    ChevronDown as chevron_down, ChevronLeft as chevron_left, ChevronRight as chevron_right, ChevronUp as chevron_up,
    ChevronsUpDown as chevrons_up_down, CircleAlert as circle_alert, CircleCheck as circle_check, CircleX as circle_x,
    CloudUpload as cloud_upload, Copy as copy, Ellipsis as ellipsis, Eye as eye, EyeOff as eye_off, Flame as flame,
    Inbox as inbox, Info as info, Lightbulb as lightbulb, Menu as menu, Minus as minus, Search as search, Star as star,
    TrendingDown as trending_down, TrendingUp as trending_up, TriangleAlert as triangle_alert, X as close,
};
