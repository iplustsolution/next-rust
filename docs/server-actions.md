# Server actions & forms

A server action is an async Rust function that browsers can invoke. It works
as a plain HTML form submission (no JavaScript needed) and as a JSON call from
the client runtime.

```rust
// src/actions.rs (or any special file in app/)
use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct Signup {
    email: String,
    name: String,
}

#[server_action]
pub async fn signup(input: Signup) -> Result<User> {
    if !input.email.contains('@') {
        return Err(Error::validation([("email", "Enter a valid email address")]));
    }
    let user = db::create_user(&input.email, &input.name).await?;
    Ok(user)
}
```

The build finds `#[server_action]` functions in the app directory and in
`src/`, and serves them at `POST /_nr/action/<id>`. The id is a hash of the
source file path and function name, so it stays stable across builds. Actions
in `src/` must be reachable as `crate::path::to::module` from the crate root.

## Signatures

```rust
#[server_action] pub async fn refresh() -> Result<Stats>
#[server_action] pub async fn save(input: Draft) -> Result<Draft>
#[server_action] pub async fn logout(ctx: ActionContext, _input: ()) -> Result<()>
```

- The input must implement `Deserialize`. Form posts are decoded as
  `application/x-www-form-urlencoded`, everything else as JSON.
- The output must implement `Serialize`.
- `ActionContext` gives access to `cookies`, `headers` and `extensions`
  (for example the authenticated user).
- On `wasm32` targets, `#[server_action]` functions are compiled out
  entirely. See [client.md](client.md#serverclient-boundary).

## Forms

```rust
use crate::actions;

pub fn Page(form: FormState) -> impl View {
    form![
        action!(actions::signup),
        label!["Email ", input![name("email"), r#type("email"), value(form.value("email"))]],
        small![data("nr-error", "email"), form.error("email").unwrap_or_default().to_owned()],
        label!["Name ", input![name("name"), value(form.value("name"))]],
        input![r#type("hidden"), name("_redirect"), value("/welcome")],
        button![r#type("submit"), "Sign up"],
    ]
}
```

`action!(path)` sets `method="post"`, the action URL, and `data-nr-action`
for enhancement.

**Without JavaScript**, the browser posts the form:

| action result | response |
|---|---|
| `Ok(_)` | `303` to `_redirect` (local paths only) or back to the same-origin referring page |
| `Err(redirect(url))` | `303` to `url` |
| `Err(Error::validation(..))` or another error | `303` back. Errors, the message, and non-sensitive submitted values are stored in a 60-second flash cookie |

The page reads them with the `FormState` extractor: `form.error("email")`,
`form.value("email")`, `form.message`, `form.has_errors()`. The flash is
consumed once. Fields named like `password` or `token`, or starting with `_`,
are never echoed back.

**With the client runtime** loaded, the form is submitted with `fetch`. The
runtime writes validation errors into elements with a matching
`data-nr-error="field"`, follows redirects client-side, and on success calls
`nextRust.refresh()` or navigates to `_redirect`. It also dispatches
`nr:success` / `nr:error` events on the form.

`FormState` makes a page dynamic. Pages that only render a form without
reading its state can stay static.

## JSON calls

```js
const user = await nextRust.action("/_nr/action/…", { email, name });
```

Responses are `{"ok":true,"data":…}`, `{"ok":false,"errors":{…}}` (422),
`{"ok":false,"error":"…"}`, or `{"ok":false,"redirect":"/…"}` (followed
automatically). From Rust, get the URL with `action!(signup).url()` and pass
it to an island. See the `server-actions` example, where an island refreshes
a counter via `data-nr-on-click="action:<url>->count"`.

## Security

- Only `POST` is accepted.
- Cross-site requests are rejected (`403`). Browsers send `Sec-Fetch-Site` /
  `Origin`, which must match the request host or `[security] allowed_origins`.
  Clients that send neither (curl, server-to-server) aren't browsers and can't
  carry a victim's cookies. Protect those calls with authentication as usual.
- `[security] csrf = "off"` disables the check. Don't.
- Validate input inside the action. Deserialization only checks shape.
- In production, internal error messages are replaced by
  "Internal Server Error". Validation messages and `Error::http` messages are
  public.
- Multipart bodies are rejected (`415`). Use an API route for uploads.
