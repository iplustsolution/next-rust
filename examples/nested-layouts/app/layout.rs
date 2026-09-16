use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Shop").title_template("%s · Shop")
}

pub fn Layout(children: Children) -> impl View {
    div![id("root-layout"), children]
}
