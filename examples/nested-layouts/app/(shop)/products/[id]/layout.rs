use next_rust::prelude::*;

pub fn Layout(children: Children, params: Params) -> impl View {
    div![id("product-layout"), data("product", params.get("id").unwrap_or_default().to_owned()), children]
}
