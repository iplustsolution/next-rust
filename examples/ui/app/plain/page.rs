use next_rust::prelude::*;
use next_rust::ui::*;

/// A page with few components: it gets only their CSS.
pub fn Page() -> impl View {
    Container![width = Width::Md, h1!["Plain"], Chip![color = Color::Success, variant = Variant::Flat, "Only chip CSS here"]]
}
