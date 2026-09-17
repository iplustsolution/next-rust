<p align="center">
  <img src="https://raw.githubusercontent.com/iplustsolution/next-rust/main/website/public/logo.svg" alt="Next Rust logo" width="160" height="160">
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

> **Early days.** Next Rust is pre-1.0 (0.0.x releases). It works: the
> examples run, the tests pass, and a fresh project builds into a single
> deployable binary. But APIs will still move before 1.0, and some pieces are
> missing. The honest list is the "Status & roadmap" page of the
> [documentation](#documentation). If something you need is on it, that's a
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

## Getting started

### 1. Install Rust (skip this if you already have it)

Next Rust needs **Rust 1.88 or newer**. To check what you have:

```sh
rustc --version
```

If that prints "command not found", or a version older than 1.88, install
Rust with [rustup](https://rustup.rs), the official installer.

**macOS and Linux:**

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:** download and run
[rustup-init.exe](https://win.rustup.rs/x86_64). If it asks, also install
the Visual Studio C++ Build Tools.

Then open a **new** terminal and confirm:

```sh
rustc --version
cargo --version
```

If you had an older Rust already, update it with `rustup update stable`.

### 2. Install the Next Rust CLI

```sh
cargo install next-rust-cli
```

This installs the `next-rust` command from crates.io, and takes a minute or
two the first time. To use the newest code from `main` instead, install from
GitHub:

```sh
cargo install --git https://github.com/iplustsolution/next-rust next-rust-cli
```

Check it worked:

```sh
next-rust --version
```

If your terminal can't find `next-rust`, make sure `~/.cargo/bin` (on Windows,
`%USERPROFILE%\.cargo\bin`) is on your `PATH`, then open a new terminal.

### 3. Create a project

```sh
next-rust new my-app
cd my-app
```

New projects use the Next Rust version published on crates.io. Pass `--git`
to track the GitHub repository instead.

Adding Next Rust to an existing Cargo project instead? `cargo add next-rust`
and `cargo add --build next-rust-build`, then see
"Installation → Adding to an existing Cargo project" in the
[documentation](#documentation).

### 4. Run it

```sh
next-rust dev
```

Open http://localhost:3000 and edit `app/page.rs`. The browser reloads when
the build finishes. If you save something broken, the last good version keeps
running and the compiler error shows up in the page.

The first build downloads and compiles the framework, so give it a moment.
After that, rebuilds are fast.

When you're ready to ship:

```sh
next-rust build   # one small, self-contained binary in .next-rust/
next-rust start
```

## Styling with Tailwind CSS

Tailwind CSS is built in. Write Tailwind classes in your views and the CSS is
generated for exactly the classes you use. There's nothing to install and no
CSS file to write: no Node.js, no npm, no `tailwind.config.js`.

```rust
pub fn Page() -> impl View {
    main![
        class("mx-auto max-w-2xl px-6 py-24"),
        h1![class("text-4xl font-bold tracking-tight dark:text-white"), "Hello"],
        a![class("btn mt-8"), href("/docs"), "Read the docs"],
    ]
}
```

New projects use it from the start (`next-rust new my-app --no-tailwind`
if you'd rather write plain CSS). In an existing project, turn it on in
`next-rust.toml`:

```toml
[tailwind]
enabled = true
dark_mode = "media"                        # or "class" / "attribute"
plugins = ["@tailwindcss/typography"]      # optional official plugins

[tailwind.theme]                           # your design tokens
color-brand = "#f26b2a"                    # → bg-brand, text-brand, ring-brand/50, …
font-display = "Inter, sans-serif"         # → font-display

[tailwind.utilities]                       # your own classes
btn = "inline-flex rounded-lg bg-brand px-4 py-2 font-semibold text-white hover:bg-brand/90"
```

How it works:

- The engine is the **official Tailwind CSS v4.3.3**, so every class and
  variant in the [Tailwind docs](https://tailwindcss.com/docs) works.
- The first time a Tailwind project builds, `next-rust dev` / `next-rust build`
  download Tailwind's standalone engine for your system (with a progress bar),
  check it against the published SHA-256 checksum, and cache it for every
  project on the machine. It needs `curl`, which macOS, Windows 10+ and most
  Linux systems already have.
- Only the classes found in `app/` and `src/` are generated. Development
  builds get readable CSS, release builds get it minified, and it's compiled
  into the binary, so servers need nothing extra.
- Release builds rename every class to a short random name, in the CSS and in
  the HTML (`mt-4 rounded-xl border` → `dx ce g`), with new names on every
  build. Development keeps the names you wrote. Turn it off with
  `minify_classes = false`, or keep single classes with `keep_classes`.
- Offline or locked-down CI? Download the executable yourself from the
  [v4.3.3 release](https://github.com/tailwindlabs/tailwindcss/releases/tag/v4.3.3)
  and set `NEXT_RUST_TAILWIND_BIN=/path/to/tailwindcss`.

Everything else (custom variants, safelisting classes built at runtime, raw
keyframes) is on the Tailwind CSS page of the [documentation](#documentation).

## Deploying

`next-rust build` produces a single file, `.next-rust/my-app`, that contains
your whole app: server, pages, `next-rust.toml` and everything in `public/`.
There are two common ways to ship it.

### Copy the binary to a server

```sh
next-rust build
scp .next-rust/my-app you@your-server:/srv/my-app
ssh you@your-server 'PORT=8080 /srv/my-app'
```

Nothing else needs to be installed on the server. Build on the same operating
system and CPU architecture as the server (for example, a Linux x86-64 binary
for a Linux x86-64 server). If you develop on a Mac and deploy to Linux, use
Docker below: it builds the Linux binary for you.

### Run it with Docker

You need [Docker](https://docs.docker.com/get-docker/) installed and running.
All commands run inside your project folder.

**1. Create the Dockerfile**

```sh
next-rust docker
```

This writes two files:

- `Dockerfile` builds your app in the official Rust image, then copies only
  the finished binary into a minimal image
  (`gcr.io/distroless/cc-debian12:nonroot`) that runs as a non-root user.
- `.dockerignore` keeps `target/`, `.next-rust/`, `.git` and local `.env`
  files out of the build.

If a `Dockerfile` already exists, the command stops instead of overwriting it.
Use `next-rust docker --force` to replace it.

**2. Build the image**

```sh
docker build -t my-app .
```

The first build downloads and compiles every dependency, so it takes a few
minutes. Later builds reuse the Cargo cache and are much faster.

**3. Run the container**

```sh
docker run -p 3000:3000 my-app
```

Open http://localhost:3000. Press Ctrl+C to stop it; the server finishes the
requests it's handling before it exits.

**Everyday commands**

| What you want | Command |
|---|---|
| Run in the background and restart after crashes or reboots | `docker run -d --name my-app --restart unless-stopped -p 3000:3000 my-app` |
| See the logs | `docker logs -f my-app` |
| Stop it | `docker stop my-app` |
| Remove the stopped container | `docker rm my-app` |
| Use another port | `docker run -p 8080:8080 -e PORT=8080 my-app` |
| Pass secrets and settings | `docker run -p 3000:3000 -e DATABASE_URL=... -e API_KEY=... my-app` |
| Pass them from a file | `docker run -p 3000:3000 --env-file .env.production my-app` |

**Good to know**

- **Rebuild after every change.** Your pages, config and `public/` files are
  compiled into the image. After editing code, run `docker build` again, then
  replace the running container.
- **`.env` files are not in the image.** Pass secrets with `-e` or
  `--env-file` when you start the container, never bake them into the image.
- **Quiet by default.** A server running in the background prints only errors.
  Run with `docker run -it ...` to see the Next Rust startup screen.
- **`-p host:container`.** The left port is the one you open in the browser;
  the right one must match `PORT` inside the container (3000 unless you change
  it).
- **Small images.** The image holds one size-optimized binary (about 1–2 MB
  for a typical app) on top of the minimal base image. There is no Rust
  toolchain, shell or package manager in it.
- **Already have a Dockerfile from an older version?** Run
  `next-rust docker --force` to get the current one.

More options (Kubernetes, reverse proxies, TLS, platforms like Fly.io and
Railway) are on the Deployment page of the [documentation](#documentation).

## Staying up to date

New projects follow this GitHub repository, so updating is one command. Run
it inside your project:

```sh
next-rust upgrade
```

That does two things:

- moves your project's framework to the latest commit on GitHub (the exact
  version stays pinned in `Cargo.lock` until you upgrade again);
- reinstalls the `next-rust` command from GitHub.

To update just one of them:

```sh
next-rust upgrade --project   # only this project's framework
next-rust upgrade --cli       # only the next-rust command
```

You don't need to remember to check. Once a day, `next-rust dev` and
`next-rust new` quietly ask GitHub whether something newer exists, and print a
short note if it does. Nothing is installed without you running
`next-rust upgrade`, so an update never lands in the middle of your work. The
check sends no information about you or your project. Turn it off with
`NEXT_RUST_NO_UPDATE_CHECK=1`.

Prefer plain Cargo? `cargo update -p next-rust -p next-rust-build` updates the
project, and running the install command from step 2 again with `--force`
updates the CLI.

### Working on the framework itself

If you want to change Next Rust while building an app with it, point the app
at your own clone instead:

```sh
git clone https://github.com/iplustsolution/next-rust
cargo install --path next-rust/crates/next-rust-cli
next-rust new my-app --framework-path ./next-rust
```

That project uses your local copy, so `git pull` in the clone updates it.

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
  Tailwind CSS v4 is built in (no Node.js), and CSS modules are scoped at
  compile time. Pages ship zero JavaScript unless
  they use client navigation or an interactive island, and even then it's a
  single ~4 KB script. Navigating between pages that share layouts renders
  and sends only the part below them; the layouts stay on screen.
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
website/               the documentation site, built with Next Rust
benchmarks/            the benchmark harness
```

To see how a request travels through all of that, read the Architecture page
of the [documentation](#documentation).

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

The documentation is a website built with Next Rust itself, in
[`website/`](website). It covers installation, routing, rendering, server
actions, deployment and every configuration option, with search. To read it
locally:

```sh
cd website
next-rust dev        # http://localhost:3000/docs
```

`next-rust build` in that folder produces the whole site as one binary, ready
to deploy. The site is pure Rust: each page is a module in
[`website/src/content/`](website/src/content) written with the same view
macros your app uses.

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
