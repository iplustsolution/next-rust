use next_rust::prelude::*;

pub fn Layout(children: Children) -> impl View {
    div![id("shop-group-layout"), children]
}
