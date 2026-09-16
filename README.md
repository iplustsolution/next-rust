<p align="center">
  <img src="docs/assets/logo.svg" alt="Next Rust logo" width="160" height="160">
</p>

<h1 align="center">Next Rust</h1>

<p align="center">
  Folders become routes. Rust becomes HTML. One binary goes to production.
</p>

```text
app/
├── layout.rs            the frame around every page
├── page.rs              /
├── about/page.rs        /about
└── blog/[slug]/page.rs  /blog/anything
```

```rust
// app/page.rs
use next_rust::prelude::*;

pub fn Page() -> impl View {
    div![
        h1!["Hello World"],
        p!["Welcome to Next Rust."]
    ]
}
```

That file is the whole route. There's no router to register it with and no
config to update. Save it, and `next-rust dev` serves it at `/`.

> **Early days.** This is version 0.1. It works: the examples run, the tests
> pass, and a fresh project builds into a deployable server. But APIs will
> still move before 1.0, and some pieces are missing. The honest list lives in
> [docs/status.md](docs/status.md). If something you need is on it, that's a
> great place to start contributing.

## The idea

If you've built a site with a file-based JavaScript framework, you already
know how this feels. You create a folder, drop in a page, wrap things in
layouts, and put an API handler next to the pages that use it.

We wanted that workflow without leaving Rust. So there's no Node.js, no
bundler, and no React. A build script reads your `app/` folder and generates
ordinary Rust code that wires everything together. The compiler checks all of
it, so a page that asks for data the framework can't give it doesn't build.
You find out at your desk instead of in production.

What comes out the other end is a single native server binary.

## A slightly bigger taste

A blog post that's pre-rendered at build time and refreshed every hour:

```rust
// app/blog/[slug]/page.rs
use next_rust::prelude::*;

pub const REVALIDATE: u64 = 3600;

pub async fn generate_params() -> Vec<Params> {
    posts::all().await.iter().map(|p| Params::new().with("slug", &p.slug)).collect()
}

pub async fn Page(params: Params) -> Result<impl View> {
    let post = posts::find(params.get("slug").unwrap_or_default()).await.or_not_found()?;
    Ok(article![h1![post.title], p![post.summary]])
}
```

An API endpoint living right next to it:

```rust
// app/api/posts/route.rs
use next_rust::prelude::*;

pub async fn GET() -> Response {
    Response::json(&posts::all().await)
}
```

And a form that works with JavaScript turned off, then gets smoother when it's on:

```rust
#[server_action]
pub async fn subscribe(input: Signup) -> Result<()> {
    if !input.email.contains('@') {
        return Err(Error::validation([("email", "That doesn't look like an email")]));
    }
    newsletter::add(&input.email).await?;
    Err(redirect("/thanks"))
}

pub fn Page(form: FormState) -> impl View {
    form![
        action!(subscribe),
        input![name("email"), value(form.value("email"))],
        small![form.error("email").unwrap_or_default().to_owned()],
        button!["Subscribe"],
    ]
}
```

## Try it

You'll need Rust 1.88 or newer.

```sh
git clone https://github.com/next-rust/next-rust
cd next-rust
cargo install --path crates/next-rust-cli

next-rust new my-app --framework-path .
cd my-app
next-rust dev
```

Open http://localhost:3000 and edit `app/page.rs`. The browser reloads when
the build finishes. If you save something broken, the last good version keeps
running and the compiler error shows up in the page.

When you're ready to ship:

```sh
next-rust build   # compiles and pre-renders what it can
next-rust start
```

The crates aren't on crates.io yet, which is why `--framework-path` points
at your checkout for now.

## What's in the box

You don't have to learn all of this on day one, but it's there when you need
it:

- **Routing:** nested layouts, route groups like `(marketing)`, dynamic and
  catch-all segments, parallel `@slots`, intercepting routes, and plain
  `page.html` files for the pages you don't want to rewrite.
- **Rendering:** server-side rendering, static generation, timed and
  on-demand revalidation, and streaming with `loading.rs`. Next Rust works
  out which pages can be static, and `next-rust routes` tells you why the
  others aren't.
- **Backend:** API routes, middleware at any level of the tree, cookies,
  sessions, CORS, rate limiting, server-sent events and WebSockets.
- **Frontend:** HTML is escaped unless you explicitly ask for raw output.
  CSS modules are scoped at compile time. Pages ship zero JavaScript unless
  they use client navigation or an interactive island, and even then it's a
  single ~4 KB script.
- **Tooling:** `new`, `dev`, `build`, `start`, `routes`, `check`, `doctor`,
  `generate` and `docker`.

Each of the 13 projects in [`examples/`](examples) comes with its own tests.
They're the quickest way to see how the pieces fit.

## How the repository is laid out

```text
crates/
├── next-rust          what applications depend on
├── next-rust-cli      the `next-rust` command
├── next-rust-build    turns app/ into generated Rust (runs in build.rs)
├── next-rust-router   scanning, validation, route matching
├── next-rust-view     the HTML macros and the streaming renderer
├── next-rust-server   HTTP, middleware, rendering pipeline, caching, actions
├── next-rust-macros   #[client], #[server_action], css_module!, asset!
├── next-rust-cache    cache stores
├── next-rust-core     config, .env loading, diagnostics
└── next-rust-assets   hashing, MIME types, CSS processing
examples/              small runnable apps, each with tests
docs/                  guides and reference
benchmarks/            the benchmark harness
```

To see how a request travels through all of that, start with
[docs/architecture.md](docs/architecture.md).

## Contributing

Next Rust is built in the open, and it only gets good if people other than
its first authors shape it. Every kind of contribution helps: a bug report, a
confusing error message you ran into, a docs fix, an example, or a whole
feature.

Some good places to jump in:

- **Automated WASM builds for client components.** The protocol exists;
  running `cargo` and `wasm-bindgen` for you doesn't yet.
- **Image resizing** and WebP output behind the existing `Image!` component.
- **Native TLS**, so small deployments don't need a proxy.
- **Browser tests** for client-side navigation and islands.
- **Windows.** Nobody has verified it yet.
- **Error messages.** If one confused you, it will confuse someone else.
  Tell us, or improve it.

Getting set up:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. It
covers how changes are proposed, what we look for in reviews, and how to
report security issues privately.

If you're unsure whether an idea fits, open an issue and ask. Questions are
contributions too.

## Documentation

The guides live in [`docs/`](docs/README.md). Start with
[getting started](docs/getting-started.md) or [routing](docs/routing.md), or
go straight to the [configuration reference](docs/configuration.md).

## License

Next Rust is available under either the [MIT license](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option. Unless you say
otherwise, any contribution you submit is licensed the same way.

Next Rust is inspired by the file-based routing popularized by Next.js. It's
an independent project, not affiliated with Vercel, and it contains no code
from Next.js or React.

---

<p align="center">
  <a href="https://www.iplust.in/">
    <img src="https://www.iplust.in/logo.png" alt="I Plus T Solution" height="48">
  </a>
</p>

<p align="center">
  Handled and managed by <a href="https://www.iplust.in/"><strong>I Plus T Solution</strong></a>
</p>
