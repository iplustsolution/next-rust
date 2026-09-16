use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Products").title_template("%s · Products · Shop")
}

pub fn Layout(children: Children) -> impl View {
    div![id("products-layout"), children]
}
