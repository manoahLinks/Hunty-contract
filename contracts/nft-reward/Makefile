default: build

all: test

test: build
	cargo test

build:
	stellar contract build
	@ls -l ../../target/wasm32-unknown-unknown/release/nft_reward.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean

