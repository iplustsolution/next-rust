use next_rust::prelude::*;

pub fn Layout(children: Children) -> impl View {
    div![class("app-shell"), children]
}
