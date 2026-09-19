# Instructions for AI coding agents

This repository is **Next Rust**, a Rust-native web framework with Next.js App Router semantics.

**Read [`skills/next-rust/SKILL.md`](skills/next-rust/SKILL.md) before making changes**, and follow it. It
explains the framework, the compile-time contracts that cannot be guessed (filenames, required export names,
signatures), how to verify your work, and how the repository itself is organised. Its `references/` folder has
the exact API surface, the full configuration reference, the framework internals and every build diagnostic.

That skill is portable: any AI tool can install it (see [`skills/next-rust/README.md`](skills/next-rust/README.md)).

Quick orientation for work in this repository:

```sh
cargo test --workspace                       # needs Rust 1.89+
cargo run -p example-basic                   # a real app on http://localhost:3000
cargo run -p next-rust-cli -- routes         # inspect a project's route table
```

Before pushing, run what CI runs:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
```

Two things that surprise newcomers: the browser runtime in
`crates/next-rust-server/src/client_runtime.rs` exists twice (readable and minified, guarded by a
source-hash test), and the documentation site is written in Rust under `website/src/content/` — there are no
Markdown documentation pages. Both are explained in
[`skills/next-rust/references/framework-internals.md`](skills/next-rust/references/framework-internals.md).

Version numbers are owned by release automation; never bump them by hand, and `git pull --rebase` before
pushing to `main`.
