use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Partial prerendering")
}

/// The layout is part of every page's static shell. Who is signed in is not:
/// that one line is a hole, rendered for each request from its cookie.
pub fn Layout(children: Children) -> impl View {
    div![
        header![
            strong!["Shop"],
            dynamic(span![class("account"), "…"], |ctx| async move {
                match ctx.cookies.get("user") {
                    Some(user) => span![class("account"), format!("Signed in as {user}")],
                    None => span![class("account"), "Not signed in"],
                }
            }),
        ],
        main![children],
    ]
}
