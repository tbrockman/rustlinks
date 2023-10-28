.PHONY: build run

build: build-web build-ui

build-web:
	cargo build

build-ui:
	cd src/ui/app && npm run build

run: build
	cargo run -- start