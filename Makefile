.PHONY: build run

build: build-web build-ui

build-web:
	cargo build

build-ui:
	cd src/ui/app && npm run build

run: build
	cargo run -- start

# install-dependencies:
# 	cargo install cargo-watch

# dev: install-dependencies
# 	cargo watch -i src/ui/app -i src/ui/dist -x 'run -- start' & \
# 	cargo watch -w src/ui/app -s 'cd src/ui/app && npm run build'

# watch-web:
# 	cargo watch -i src/ui/app -i src/ui/dist -x 'run -- start'

# watch-ui:
# 	cargo watch -w src/ui/app -s 'cd src/ui/app && npm run build'
