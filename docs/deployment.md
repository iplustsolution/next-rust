# Deployment

A Next Rust application is **one native binary** plus its build output
(`.next-rust/`) and `public/`. It needs no runtime, interpreter or Node.js.

## What to ship

```text
my-app                      # the server binary (target/release/my-app or .next-rust/server/my-app)
next-rust.toml              # if you have one
public/                     # static files
.next-rust/cache/pages/     # pre-rendered pages (static routes render on demand if missing)
client/, assets/            # if you use module islands / asset!
```

Environment: `NEXT_RUST_ENV=production`, `PORT`, `HOST`, and your secrets.

## Bare metal / VPS

```sh
next-rust build
scp -r .next-rust/server/my-app next-rust.toml public .next-rust server:/srv/my-app/
ssh server 'cd /srv/my-app && NEXT_RUST_ENV=production PORT=8080 ./my-app'
```

A systemd unit:

```ini
[Unit]
Description=my-app
After=network.target

[Service]
WorkingDirectory=/srv/my-app
ExecStart=/srv/my-app/my-app
Environment=NEXT_RUST_ENV=production PORT=8080
EnvironmentFile=-/srv/my-app/.env.production.local
Restart=on-failure
DynamicUser=yes
StateDirectory=my-app

[Install]
WantedBy=multi-user.target
```

The server shuts down gracefully on `SIGTERM`. It stops accepting
connections and waits up to `[server] shutdown_timeout` seconds for in-flight
requests.

## Docker

```sh
next-rust docker
docker build -t my-app .
docker run -p 3000:3000 my-app
```

The generated Dockerfile:

- builds in `rust:1-slim` with BuildKit cache mounts for the registry and target directory;
- runs static generation during the image build;
- copies only the binary, `public/`, the config and `.next-rust/` into
  `debian:bookworm-slim`, running as an unprivileged user.

## Kubernetes

- Liveness and readiness: add a lightweight route, e.g.
  `App::new(routes()).get("/healthz", |_| async { "ok" })`.
- `terminationGracePeriodSeconds` should exceed `[server] shutdown_timeout`.
- For more than one replica, use a shared `CacheStore` (see
  [caching](caching.md)) so ISR revalidation and `revalidate_tag` affect every
  replica. With the default file store, each pod has its own page cache,
  seeded from the image.
- The built-in rate limiter and `MemorySessionStore` are per pod.

## Fly.io, Railway, Render and similar platforms

These platforms detect a Dockerfile. `PORT` is honoured automatically. Set
`NEXT_RUST_ENV=production` and `[server] trust_proxy = true` so client IPs
and the scheme come from the platform's proxy.

## Reverse proxies, TLS and HTTP/3

The built-in server speaks HTTP/1.1 and HTTP/2 (cleartext, via prior
knowledge or a proxy). Terminate **TLS** and **HTTP/3** at a reverse proxy or
load balancer (Caddy, nginx, Envoy, a cloud load balancer). This is the usual
production setup and keeps certificate management out of the application.

```caddyfile
example.com {
    reverse_proxy 127.0.0.1:3000
}
```

With a proxy in front:

- set `[server] trust_proxy = true`;
- set `[security] hsts_max_age = 31536000` once the site is HTTPS-only;
- leave `[server] compression = true` unless the proxy compresses. Streamed
  HTML flushes per chunk either way, but proxies must not buffer it. Nginx
  honours the `x-accel-buffering: no` header the framework sends for SSE;
  for streamed HTML, set `proxy_buffering off` on those locations.

Native TLS and HTTP/3 in the server are on the roadmap
([status.md](status.md)).

## Serverless and edge

The framework core is independent of the listener. `App::handle(Request)`
takes a request and returns a response:

```rust
next_rust::routes!();

// Example adapter for any platform that gives you an http::Request<Bytes>.
pub async fn handle(req: http::Request<bytes::Bytes>) -> http::Response<Vec<u8>> {
    static APP: std::sync::OnceLock<next_rust::App> = std::sync::OnceLock::new();
    let app = APP.get_or_init(|| next_rust::App::new(routes()).build().expect("app"));
    let res = app.handle(next_rust::Request::from_http(req)).await;
    let (status, headers) = (res.status, res.headers.clone());
    let body = res.into_bytes().await.unwrap_or_default().to_vec();
    let mut out = http::Response::new(body);
    *out.status_mut() = status;
    *out.headers_mut() = headers;
    out
}
```

Wrap this in `lambda_http`, a Cloudflare Workers WASM shim, or a similar
adapter. Constraints on serverless platforms:

- the process may be frozen between requests, so background ISR
  regeneration may be delayed. Prefer on-demand `revalidate_tag`;
- configure a `CacheStore` backed by the platform's KV store;
- edge runtimes compiled to `wasm32` can't use Tokio's networking or the
  filesystem store. The request pipeline itself doesn't need them, but no
  official edge adapter ships yet.

## Observability

- Logs: one JSON object per line in production
  (`{"level":"info","msg":"request","method":"GET","path":"/","status":"200","duration_ms":"1.42"}`),
  pretty in development. Configure with `[logging]`.
- `request_id()` middleware adds `x-request-id`, and error digests appear in
  both the log and the error UI.
- With the `tracing` feature, all framework log events go through `tracing`,
  so any subscriber (OpenTelemetry, Datadog, JSON) collects them.
- Secrets aren't logged. The framework logs method, path, status and
  duration, never headers, cookies or bodies.
