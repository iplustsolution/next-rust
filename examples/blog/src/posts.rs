/// A blog post. In a real application this would come from a database or
/// Markdown files.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Post {
    pub slug: &'static str,
    pub title: &'static str,
    pub date: &'static str,
    pub summary: &'static str,
    pub paragraphs: &'static [&'static str],
}

pub const POSTS: &[Post] = &[
    Post {
        slug: "hello-next-rust",
        title: "Hello, Next Rust",
        date: "2026-09-01",
        summary: "Filesystem routing, layouts and streaming — in Rust.",
        paragraphs: &["Every page.rs file is a route.", "Layouts wrap their children automatically."],
    },
    Post {
        slug: "static-generation",
        title: "Static generation",
        date: "2026-09-10",
        summary: "Pre-render pages at build time and revalidate them later.",
        paragraphs: &[
            "Pages without request data are rendered at build time.",
            "REVALIDATE refreshes them in the background.",
        ],
    },
];

pub fn find(slug: &str) -> Option<&'static Post> {
    POSTS.iter().find(|p| p.slug == slug)
}
