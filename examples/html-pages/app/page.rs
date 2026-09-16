use next_rust::prelude::*;

pub fn Page() -> impl View {
    ul![li![a![href("/legacy"), "Legacy fragment"]], li![a![href("/landing"), "Full HTML document"]]]
}
