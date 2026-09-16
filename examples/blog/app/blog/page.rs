use next_rust::prelude::*;

use crate::posts::POSTS;

pub fn metadata() -> Metadata {
    Metadata::new().title("Archive")
}

pub fn Page() -> impl View {
    section![h1!["Archive"], each(POSTS, |p| article![h2![Link!(href = format!("/blog/{}", p.slug), p.title)], p![p.summary]])]
}
