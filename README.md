# rustlinks

a work-in-progress Rust implementation of [golinks](https://golinks.github.io/golinks/)

## features

- [x] no upfront network requests for link resolution
- [ ] browser ui
- [x] metrics
- [x] traces
- [x] tls
  - [ ] sni
- [ ] sso

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
docker-compose up -d
# start the rustlinks server locally
cargo run -- start
```

## tls

install [mkcert](https://github.com/FiloSottile/mkcert#installation) (if you don't already have a certificate authority)

```shell
mkcert -install
mkcert -key-file key.pem -cert-file cert.pem rs
cargo run -- start --cert cert.pem --key key.pem
```

## architecture

- an entry in /etc/hosts to direct requests to `https://rs` to the locally running `rustlinks` server
- a local CA to provide valid certificates for the above
- an `actix-web`-based Rust application, maintaining an in-memory set of links and shortened aliases (persisted to disk on modifications) by watching a namespace in `etcd`
- `open-telemetry` OTLP formatted metrics and traces, for analytics and observability
- `clickhouse` for storage of metrics and traces
- `grafana` for visualization of metrics and traces

## todo

- [ ] a React UI for creating, searching, and deleting link aliases
- [ ] OAuth + SSO integration
- [ ] limit link storage (to not break `etcd` or unnecessarily store links which likely won't be used)
- [ ] distinguish readers vs. writers
  - [ ] writers manage the `rustlinks` `etcd` namespace (e.g. adds/removes links)
  - [ ] readers watch the `rustlinks` `etcd` namespace for changes
- [ ] allow installation as service
- [ ] benchmarking/load test
  - [ ] what happens if we insert 100k aliases and then start the program
  - [ ] what happens if we insert 100k aliases while the program is running
