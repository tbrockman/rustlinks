# rustlinks

a work-in-progress Rust implementation of [golinks](https://golinks.github.io/golinks/)

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

## architecture

- an entry in /etc/hosts to direct requests to `https://rs` to the locally running `rustlinks` server
- a local CA to provide valid certificates for the above
- an `actix-web`-based Rust application, maintaining an in-memory set of links and shortened aliases (persisted to disk on modifications) by watching a namespace in `etcd`
- `open-telemetry` OTLP formatted metrics and traces, for analytics and observability

## todo

- [ ] a React UI for creating, searching, and deleting link aliases
- [ ] OAuth + SSO integration
- [ ] limit link storage (to not break `etcd` or unnecessarily store links which likely won't be used)
