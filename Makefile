run:
	cargo run --release -- -p $(path)

build:
	cargo build

test:
	cargo test

format:
	cargo fmt

format-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy

2048:
	cargo run --release -- -p example_images/2048.obj

rogue:
	cargo run --release -- -p example_images/rogue.obj
