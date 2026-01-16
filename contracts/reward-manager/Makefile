default: build

all: test

test: build
	cargo test

build:
	stellar contract build
	@ls -l ../../target/wasm32-unknown-unknown/release/reward_manager.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean

