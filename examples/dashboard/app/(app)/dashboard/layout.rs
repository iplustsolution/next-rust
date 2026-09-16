use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Dashboard")
}

pub fn Layout(children: Children, mut slots: Slots, cookies: Cookies) -> impl View {
    let user = cookies.get_decoded("user").unwrap_or_default();
    div![
        aside![
            p![format!("Signed in as {user}")],
            nav![Link!(href = "/dashboard", "Overview"), Link!(href = "/dashboard/settings", "Settings"), Link!(href = "/dashboard/reports", "Reports")],
        ],
        section![class("content"), children],
        section![class("analytics"), slots.take("analytics")],
        section![class("activity"), slots.take("activity")],
    ]
}
