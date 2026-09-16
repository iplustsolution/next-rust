use next_rust::prelude::*;

pub fn Template(children: Children) -> impl View {
    div![class("settings-template"), children]
}
