# Security

This page describes the framework's defaults and reports the pre-1.0 audit of
each area named in the project requirements. "Tested" means an automated
test in this repository covers the behaviour.

## Defaults at a glance

| area | default |
|---|---|
| HTML output | all text and attributes escaped; `raw_html` is explicit |
| URL attributes | `javascript:`, `vbscript:`, non-image `data:` neutralized |
| Event handler attributes | dropped unless `raw_attr` |
| Response headers | `X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN`, `Referrer-Policy: strict-origin-when-cross-origin`, `Cross-Origin-Opener-Policy: same-origin`, restrictive `Permissions-Policy` |
| CSP | opt-in via `[security] csp`; framework inline scripts always carry a per-request nonce |
| Cookies | `HttpOnly`, `SameSite=Lax`, `Path=/`, `Secure` outside development |
| Server actions | POST-only, cross-site requests rejected |
| Request bodies | 2 MiB limit; header read timeout 10 s; request timeout 60 s |
| Errors | details only in development; production shows a digest |
| Static files | traversal-safe resolver, no dotfiles, no directory listings |
| Secrets | only `NEXT_RUST_PUBLIC_*` variables can reach clients |

## Audit

### Path traversal / directory traversal / static file serving — **pass (tested)**

`static_files::resolve_safe` percent-decodes each segment. It rejects `.`,
`..`, empty segments, segments starting with `.`, backslashes, NUL bytes and
`:`, before touching the filesystem. It then canonicalizes the result and
requires it to stay under the canonical root, which defeats symlink escapes.
Only regular files are served.

Tests cover `/../`, `%2e%2e`, `..%2F`, backslash, NUL, dotfiles, directories
and symlinks pointing outside the root (`static_files::tests`,
`tests/pipeline.rs::public_files`). The same resolver serves `/_nr/client/`,
`/_nr/assets/` and `/_nr/image`.

### HTML escaping, template injection, XSS — **pass (tested)**

- The DSL escapes text (`& < >`) and attribute values (`& < > " '`).
  Templates are Rust code, not strings, so there is no template language to
  inject into.
- URL attributes are checked by `is_safe_url`, which ignores control
  characters and whitespace so `java\tscript:` is caught.
- `attr("onclick", ..)` is dropped. Attribute names with quotes, spaces,
  `=`, `/` or `>` are dropped.
- Text inside `<script>`/`<style>` is escaped against `</script` breakouts.
- Metadata values are escaped. Unsafe canonical/link URLs become `#`.
- Island props are JSON inside an attribute, escaped as attribute text.
- Streaming redirect scripts embed the location via JSON encoding with `</`
  escaped.

Residual risk: `raw_html`, `raw_attr`, plugin `head()` markup and the
contents of `script!` elements you write are trusted by design.

### CSRF — **pass for server actions (tested); opt-in elsewhere**

Server actions check `Sec-Fetch-Site`/`Origin` against `Host`. Plain API
routes aren't automatically CSRF-protected, because many are called
cross-origin by design. Use `csrf()` middleware (double-submit token,
constant-time comparison) or SameSite cookies as appropriate.

### SSRF — **pass**

The framework makes no outbound requests on behalf of clients. The image
endpoint accepts only local paths starting with `/` (not `//`) and resolves
them inside `public/`. Remote `Image!` sources are emitted as plain `src`
attributes and never fetched by the server.

### Request smuggling — **delegated to hyper**

HTTP parsing, `Content-Length`/`Transfer-Encoding` handling and connection
management are done by hyper 1.x, which rejects ambiguous framing. The
framework doesn't parse HTTP itself. Deploy behind proxies that normalize
requests, the standard advice for any origin server.

### Header injection — **pass (tested)**

All header writes go through `http::HeaderValue`, which rejects CR/LF.
`Response::set_header` drops invalid values and logs a warning. Redirect
locations use the same validation. Cookie names, values and attributes are
validated (`cookies::tests`). SSE event names and ids have newlines stripped.

### Cookie security — **pass (tested)**

Defaults are `HttpOnly; SameSite=Lax; Path=/`, plus `Secure` outside
development. `SameSite=None` always adds `Secure`. Session ids are 256 bits
from the OS CSPRNG (`getrandom`). Session data is server-side, and
`Session::regenerate` exists to prevent fixation. Flash cookies used for form
errors expire after 60 seconds and never contain password- or token-like
fields.

### Secret exposure / client bundle leakage — **pass, with caveats**

- Server code is compiled only into the server binary. The framework doesn't
  build client bundles from application code.
- `#[server]` and `#[server_action]` items are compiled out on `wasm32`, so
  WASM client crates can't accidentally include them.
- Environment variables reach HTML only if prefixed with `NEXT_RUST_PUBLIC_`,
  and only on pages with islands.
- Caveats: `#[client]` component props are serialized into the page, so
  never pass secrets as props. Anything you render into HTML is public.
- Static pages that read request data fail to render (`DynamicUsage`)
  instead of caching one user's data for everyone (tested).

### File uploads — **limited by design**

Bodies are capped (`body_limit`, `Content-Length` checked early, streaming
limit enforced during reads). Server actions reject multipart. No upload
parser is bundled. Use `req.take_body()` with a vetted multipart crate in an
API route, store files outside `public/`, and validate type and size.

### Denial of service — **basic protections**

- header read timeout (slowloris), request timeout, graceful shutdown;
- body size limits;
- bounded caches (`MemoryStore` LRU) and a bounded rate-limit map;
- ISR regeneration deduplicated per path;
- the dev-only endpoints (`/_nr/dev/*`) are disabled outside development (tested).

Not provided: connection limits per IP, or distributed rate limiting. Use a
proxy or CDN.

### Error information leakage — **pass (tested)**

In production, internal errors render a generic page with a random digest
that is also logged. Details (messages, source file) appear only in
development (`tests/pipeline.rs::error_boundaries_and_global_errors`).

## Reporting vulnerabilities

Please report security issues privately to the maintainers before
disclosure. Until a dedicated address exists, open a GitHub security
advisory on the repository.
