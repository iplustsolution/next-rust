use std::sync::atomic::Ordering;

use next_rust::prelude::*;

pub const REVALIDATE: u64 = 60;
pub const TAGS: &[&str] = &["stats"];

pub async fn Page() -> Result<impl View> {
    let render = crate::RENDERS.fetch_add(1, Ordering::SeqCst) + 1;
    // Data is memoized in the data cache and shares the "stats" tag.
    let stars: u32 = next_rust::cache("stars", next_rust::CacheOptions::revalidate(60).tag("stats"), || async {
        Ok::<_, Error>(4_200)
    })
    .await?;
    Ok(div![h1!["Static page"], p![format!("Stars: {stars}")], p![class("render"), format!("render {render}")]])
}
