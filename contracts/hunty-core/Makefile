default: build

all: test

test: build
	cargo test

build:
	stellar contract build
	@ls -l ../../target/wasm32-unknown-unknown/release/hunty_core.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean

