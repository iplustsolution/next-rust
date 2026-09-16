use next_rust::prelude::*;

use crate::posts::POSTS;

pub async fn sitemap() -> Sitemap {
    POSTS.iter().fold(Sitemap::new().url("https://blog.example.com/"), |s, p| {
        s.url(format!("https://blog.example.com/blog/{}", p.slug))
    })
}
