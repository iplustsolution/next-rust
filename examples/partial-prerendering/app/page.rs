use std::sync::atomic::Ordering;
use std::time::Duration;

use next_rust::prelude::*;
use next_rust::{CacheOptions, tokio};

/// A static shell with per-request holes. `auto` finds this out on its own
/// (the file calls `dynamic`); saying it makes the build check that nothing
/// outside the holes reads the request.
pub const RENDERING: Rendering = Rendering::Partial;
/// The shell is cached like a static page: an hour, or until
/// `revalidate_tag("products")`.
pub const REVALIDATE: u64 = 3600;
pub const TAGS: &[&str] = &["products"];

/// A slow per-visitor lookup: it delays its hole, not the page.
async fn recommendations(user: Option<String>) -> Vec<String> {
    tokio::time::sleep(Duration::from_millis(150)).await;
    let who = user.unwrap_or_else(|| "new visitors".into());
    vec![format!("Picked for {who}: walnut desk"), format!("Picked for {who}: brass lamp")]
}

pub async fn Page() -> Result<impl View> {
    let renders = crate::SHELL_RENDERS.fetch_add(1, Ordering::SeqCst) + 1;
    // Rendered once and reused wherever the key appears, until its tag is revalidated.
    let reviews = cached("reviews:chair", CacheOptions::revalidate(300).tag("reviews"), || async {
        ul![class("reviews"), li!["“Sturdy.” ★★★★★"], li!["“Arrived early.” ★★★★"]]
    })
    .await?;
    Ok(article![
        h1!["Oak chair"],
        p![class("shell"), format!("Shell render #{renders}")],
        reviews,
        h2!["For you"],
        dynamic(p![class("placeholder"), "Finding things you may like…"], |ctx| async move {
            let items = recommendations(ctx.cookies.get("user")).await;
            ul![class("picks"), each(items, |item| li![item])]
        }),
    ])
}
