use next_rust::prelude::*;

use crate::posts::POSTS;

pub fn Page() -> impl View {
    section![
        h1!["Latest posts"],
        ul![each(POSTS, |post| li![
            key(post.slug),
            Link!(href = format!("/blog/{}", post.slug), post.title),
            " ",
            time![datetime(post.date), post.date],
        ])],
    ]
}
