# Contributing to Next Rust

Thanks for taking the time. Whether you're fixing a typo or rewriting the
renderer, here's what you need to know.

## Before you start

- **Small fixes** (typos, docs, obvious bugs): just open a pull request.
- **Bigger changes** (new features, public API changes, new dependencies):
  open an issue first and describe what you'd like to do. Nobody wants you to
  spend a weekend on something that won't be merged.
- **Not sure?** Ask in an issue. There are no silly questions here.

## Setting up

```sh
git clone https://github.com/iplustsolution/next-rust
cd next-rust
cargo test --workspace
```

You need Rust 1.88 or newer. Nothing else is required.

To try your changes in a real app, the examples are the fastest route:

```sh
cargo test -p example-dashboard
cargo run -p example-basic      # then open http://localhost:3000
```

Or create a scratch project against your checkout:

```sh
cargo run -p next-rust-cli -- new /tmp/try-it --framework-path "$PWD"
```

## Where things live

`docs/architecture.md` explains how the crates fit together. The short
version:

- A routing bug? Look in `crates/next-rust-router`. Its tests build real
  directory trees in `tests/routing.rs`.
- Wrong generated code or a missing diagnostic? That's
  `crates/next-rust-build`.
- A request being handled wrongly? Start at `dispatch` in
  `crates/next-rust-server/src/app.rs`.
- HTML output? `crates/next-rust-view`.

## Changing the browser runtime

The client runtime lives in `crates/next-rust-server/src/client_runtime.rs`
as `RUNTIME_JS` (readable, served in development) and `RUNTIME_JS_MIN`
(minified, served in production). If you edit `RUNTIME_JS`, regenerate the
minified copy with [Terser](https://terser.org), run from a scratch directory
outside the repository:

```sh
npx terser runtime.js --module --compress passes=3 --mangle toplevel=true -o runtime.min.js
```

Paste the output into `RUNTIME_JS_MIN` and update `RUNTIME_JS_SOURCE_HASH`
(the failing test prints the expected value). The tests check that the two
stay in sync and that the minified copy keeps the public attribute and header
names.

## What a good pull request has

- **A test.** Framework behaviour is easy to break by accident. For a bug,
  add a test that fails without your fix.
- **Docs, if behaviour changed.** Update the relevant page in `docs/`, and
  `docs/status.md` if a feature moves from missing to done.
- **Passing checks.** Run these before you push:

  ```sh
  cargo fmt --all
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  ```

- **A clear description.** Say what changed and why. If you picked one of
  several possible approaches, a sentence on why helps reviewers a lot.

## A few project values

- **Error messages are features.** A new failure mode deserves a readable
  diagnostic with a hint about how to fix it.
- **Dependencies need a reason.** Explain any new crate in the PR, and add it
  to the table in `docs/architecture.md`.
- **Say what doesn't work.** If a feature is partial, document the gap
  rather than implying it's complete.
- **Secure by default.** Escaping, cookie defaults and path handling don't
  get weaker for convenience.

## Reporting bugs

Include the Next Rust version, your Rust version, your OS, and the smallest
`app/` layout that reproduces the problem. Output from `next-rust doctor`
and `next-rust routes --layouts` is often enough to find the cause.

## Security issues

Please **don't** open a public issue for a vulnerability. Report it privately
through GitHub's security advisory feature on the repository, so it can be
fixed before it's disclosed.

## Licensing

Next Rust is dual-licensed under MIT and Apache-2.0. Unless you state
otherwise, any contribution you intentionally submit is licensed the same
way, with no additional terms.

## Be kind

Assume good intent, review the code rather than the person, and remember
that everyone here started out as a beginner once.
