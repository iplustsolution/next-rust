use next_rust::prelude::*;

pub fn robots() -> Robots {
    Robots::allow_all().sitemap("https://blog.example.com/sitemap.xml")
}
