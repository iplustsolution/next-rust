# Getting started

## Requirements

- Rust **1.88** or newer (`rustup update stable`)
- macOS, Linux or Windows

Next Rust does not need Node.js.

## Install the CLI

```sh
cargo install --git https://github.com/iplustsolution/next-rust next-rust-cli
```

This installs the `next-rust` binary. Don't have Rust yet? See the
[installation steps in the README](../README.md#getting-started).

## Create a project

```sh
next-rust new my-app
cd my-app
next-rust dev
```

Open <http://localhost:3000>. Edit `app/page.rs` and save: the dev server
rebuilds and the browser reloads.

New projects depend on the GitHub repository. Update the project and the CLI
with `next-rust upgrade`; see [the CLI reference](cli.md#upgrade). To develop
against a local clone instead, use
`next-rust new my-app --framework-path /path/to/next-rust`.

## What was generated

```text
my-app/
├── Cargo.toml          # depends on next-rust, build-depends on next-rust-build
├── build.rs            # fn main() { next_rust_build::generate(); }
├── next-rust.toml      # optional configuration
├── src/main.rs         # next_rust::app!();
├── public/
│   ├── favicon.svg     # the Next Rust logo (replace with yours)
│   └── robots.txt
├── .env.example
└── app/
    ├── layout.rs       # root layout and metadata
    ├── globals.css     # styles (dark and light)
    ├── page.rs         # / — a single hero to replace with your own
    ├── not-found.rs    # 404 page
    └── api/hello/route.rs  # GET /api/hello
```

Three pieces connect your files to the framework:

1. **`build.rs`** runs `next_rust_build::generate()`. It scans `app/`,
   validates the routes, reads the signatures of your special files, and
   writes Rust code to `$OUT_DIR/next_rust_routes.rs`.
2. **`src/main.rs`** calls `next_rust::app!()`. This includes the generated
   code and defines `main`.
3. **Special files** like `page.rs` and `layout.rs` are ordinary Rust
   modules. You never register them anywhere.

## Your first page

```rust
// app/products/page.rs
use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Products")
}

pub fn Page() -> impl View {
    section![
        h1!["Products"],
        ul![each(["Keyboard", "Mouse"], |name| li![name])],
    ]
}
```

That's the whole route: `/products` now exists.

A dynamic route:

```rust
// app/products/[id]/page.rs
use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct P { id: u64 }

pub fn Page(Path(p): Path<P>) -> impl View {
    h1![format!("Product #{}", p.id)]
}
```

An API endpoint:

```rust
// app/api/time/route.rs
use next_rust::prelude::*;

pub async fn GET() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "unix": std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }))
}
```

## Inspect routes

```sh
$ next-rust routes --layouts
METHOD  PATH            RENDER      SOURCE
GET     /               ○ static    app/page.rs
          └ app/layout.rs
GET     /products/:id   λ dynamic   app/products/[id]/page.rs
          └ app/layout.rs
          dynamic because: dynamic segment without generate_params
GET     /api/time       api         app/api/time/route.rs
```

## Build and run for production

```sh
next-rust build    # release build + static generation → .next-rust/
next-rust start    # run it
```

`next-rust build` prints which pages were pre-rendered (`○`), which render
per request (`λ`) and which are API routes (`ƒ`). You can also run
`./target/release/my-app` directly from any working directory.

## Adding to an existing Cargo project

```toml
# Cargo.toml
[dependencies]
next-rust = "0.1"

[build-dependencies]
next-rust-build = "0.1"
```

```rust
// build.rs
fn main() { next_rust_build::generate(); }
```

```rust
// src/main.rs
next_rust::app!();
```

Then create `app/page.rs`. Use `next_rust::routes!()` instead of `app!()`
when you want your own `main` (custom middleware, background jobs, a
library + binary split for testing). See [testing.md](testing.md).

Next: [Routing](routing.md).
