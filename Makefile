run:
	cargo run $(path)

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
	cargo run example_images/2048.obj

rogue:
	cargo run example_images/rogue.obj
