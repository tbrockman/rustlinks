# rustlinks

a work-in-progress Rust implementation of [golinks](https://golinks.github.io/golinks/)

## the idea

adding additional network hops to resolve link aliases at the scale that most companies use them is unnecessary, and in a remote/global workforce can be a significant source of latency depending on where those servers reside.

instead, we can synchronize changes in the background, and resolve links locally, without the need for waiting on network requests.

## features

- [x] links synchronized and stored locally
- [ ] oidc client
- [ ] browser ui
- [x] metrics
- [x] traces
- [ ] tls
  - [x] single cert
  - [ ] sni (see: https://stephanheijl.com/rustls_sni.html)
- [ ] search index
- [ ] automatic link pruning
  - [ ] prunes links which return 404s
  - [ ] prunes links which haven't been used in a while

## development

### clone repository and submodules

```shell
git clone --recurse-submodules https://github.com/tbrockman/rustlinks
```

### build

```shell
cargo build
```

### run

```shell
# start etcd, otel collector, clickhouse, and grafana
docker compose up -d
# start the rustlinks server locally
cargo run -- start
```

## tls

install [mkcert](https://github.com/FiloSottile/mkcert#installation) (if you don't already have a certificate authority)

```shell
mkcert -install
mkcert -key-file key.pem -cert-file cert.pem rs # [...and any other hostnames]
cargo run -- start --cert-file cert.pem --key-file key.pem --port 443
```

## architecture

- an entry in /etc/hosts to direct requests to `https://rs` to the locally running `rustlinks` server
- a local CA to provide valid certificates for the above
- an `actix-web`-based Rust application, maintaining an in-memory set of links and shortened aliases (persisted to disk on modifications) by watching a namespace in `etcd`
- `open-telemetry` OTLP formatted metrics and traces, for analytics and observability
- `clickhouse` for storage of metrics + traces
- `grafana` for visualization of metrics + traces

## todo

- [ ] tests: CLI, unit, and integration tests
- [ ] a React UI for CRUD'ing link aliases
- [ ] configurable URL fallback
- [ ] OAuth
- [ ] limit link storage (to not break `etcd` or unnecessarily store links which likely won't be used)
- [ ] distinguish readers vs. writers
  - [ ] writers manage the `rustlinks` `etcd` namespace (e.g. adds/removes links)
  - [ ] readers watch the `rustlinks` `etcd` namespace for changes
- [ ] make service/daemon installation simpler
- [ ] benchmarking/load test
  - [ ] what happens if we insert 100k aliases and then start the program
  - [ ] what happens if we insert 100k aliases while the program is running
- [ ] look into potentially replacing oidc clients in rust+ts (`openidconnect` and `oidc-client-ts`, respectively) (is the convenience worth the added deps?)
