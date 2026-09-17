# CLI

```text
next-rust <COMMAND> [OPTIONS]
```

The CLI depends only on the standard library and the framework's own crates
(no argument-parsing library, no async runtime). It drives `cargo` for
compilation.

## `new <name>`

Creates a project: `Cargo.toml`, `build.rs`, `src/main.rs`, `next-rust.toml`,
`app/` with a layout, two pages, a 404 page and an API route, plus `public/`,
`.gitignore` and `.env.example`.

By default the project depends on
`https://github.com/iplustsolution/next-rust`.

| option | |
|---|---|
| `--framework-path <dir>` | depend on a local Next Rust checkout (path dependencies) instead |

## `dev`

Development server:

- validates routes on every change and prints readable diagnostics;
- rebuilds incrementally with `cargo build`. While it compiles, and after a
  compile error, **the previous server keeps serving**;
- restarts the server after a successful build. Connected browsers
  reconnect and reload automatically;
- sends compile errors to the browser as an overlay
  (`.next-rust/dev/status.json`, streamed via `/_nr/dev/events`);
- reloads the browser without a rebuild when `public/` or `client/` change;
- prints `route added` / `route removed` when files appear or disappear;
- **fills new empty special files with starter code.** Create an empty
  `app/about/page.rs` and it becomes a working `/about` page within a
  moment. The same goes for `layout.rs`, `route.rs`, `loading.rs`,
  `error.rs`, `not-found.rs`, `middleware.rs`, `template.rs`,
  `metadata.rs` and `default.rs`. Only brand-new, empty files are filled:
  files with content, files you empty later, and files in `_private`
  folders are never touched. Turn it off with `[dev] scaffold = false`.

Watched for rebuilds: the app directory, `src/`, `assets/`, `build.rs`,
`Cargo.toml`, the config file, `[api] directory` and `[dev] watch`. Watching
uses polling (`[dev] poll_interval`), which works the same on every
platform, over network filesystems and in containers.

| option | |
|---|---|
| `--port, -p <port>` | port (default from config) |

## `build`

1. validates routes (fails on errors);
2. `cargo build --release`;
3. copies the binary to `.next-rust/server/<name>`;
4. runs it with `--export` to pre-render static pages into
   `.next-rust/cache/pages` and `.next-rust/static`;
5. writes `.next-rust/manifest/routes.json` (route manifest with rendering
   modes and pre-rendered params) and `manifest/build.json`;
6. prints a summary:

```text
Route                                              Size
○ /                                               705 B  1 page
○ /blog/:slug                                     2.1 kB  12 pages · revalidate 3600s
ƒ /api/hello                                             api
λ /products/:id                                          dynamic segment without generate_params

○  static (pre-rendered)   λ  dynamic (server-rendered)   ƒ  API
```

| option | |
|---|---|
| `--no-export` | skip static generation |

## `start`

Runs `.next-rust/server/<name>` with `NEXT_RUST_ENV=production`.
`--port` overrides the port.

## `check`

Validates routes and special-file exports, then runs `cargo check`.

## `routes`

Prints the route table.

| option | |
|---|---|
| `--layouts` | layout chain and the reason a route is dynamic |
| `--tree` | the scanned directory tree with special files |
| `--json` | the route manifest (same format as `routes.json`) |

## `analyze`

Reads the last build's manifests: static/dynamic/API counts, pre-rendered
page sizes (largest first) with render times, on-demand routes, and binary
size.

## `generate <kind> <route>`

Scaffolds a special file. Kinds: `page`, `layout`, `template`, `loading`,
`error`, `not-found`, `api`, `middleware`.

```sh
next-rust generate page /products/[id]
next-rust generate api /api/orders
next-rust generate loading /dashboard
```

## `editor`

Adds editor support for VS Code:

- `.vscode/next-rust.code-snippets`: type a prefix and pick the suggestion.
  `nrpage`, `nrparams`, `nrlayout`, `nrroute`, `nrget`, `nrpost`, `nrmeta`,
  `nrload`, `nraction`, `nrform`, `nrloading`, `nrerror`, `nrnotfound`,
  `nrmiddleware`, `nrcomponent`, `nrlink`, `nrimage`, `nrcss`.
- `.vscode/extensions.json`: recommends rust-analyzer, which provides
  completion for everything in `next_rust::prelude` (element macros,
  attributes, extractors, `Response`, …).
- `.vscode/settings.json`: re-runs build scripts on save, so a page you just
  created is part of the crate, and gets completions, right away.

New projects include these files. Existing files are kept unless `--force`.

## `doctor`

Checks the toolchain (rustc ≥ 1.88), configuration, `build.rs` and
`src/main.rs` wiring, the `next-rust-build` dependency, `.env` git-ignore,
port availability, and route diagnostics.

## `docker`

Writes a multi-stage `Dockerfile` and `.dockerignore`. See
[deployment](deployment.md). `--force` overwrites existing files.

## `upgrade`

Updates to the latest Next Rust on GitHub.

| option | |
|---|---|
| *(none)* | both of the below |
| `--project` | `cargo update -p next-rust -p next-rust-build` in the current project, reporting the old and new commit |
| `--cli` | `cargo install --git https://github.com/iplustsolution/next-rust next-rust-cli --force` |

Projects that use `--framework-path` are updated with `git pull` in that
checkout. `upgrade` says so instead of changing anything.

### Update notice

Once every 24 hours, `dev` and `new` check GitHub for a newer version: the
workspace version in `Cargo.toml`, and the latest commit compared with your
`Cargo.lock`. If something is newer, they print a note suggesting
`next-rust upgrade`. The check:

- never installs or changes anything;
- sends nothing about you or your project (it downloads one public file and
  runs `git ls-remote`);
- gives up silently after about 5 seconds or on any error;
- is skipped when `CI` or `NEXT_RUST_NO_UPDATE_CHECK` is set.

The time of the last check is stored in `~/.next-rust/last-update-check`.

## `clean`

Removes the build output directory. It refuses to delete a directory outside
the project.

## Exit codes

`0` success, `1` failure (diagnostics were printed).
