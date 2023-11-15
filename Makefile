check::
	cargo clippy --all
	cargo fmt --all

check-enforce::
	cargo clippy --all -- -D warnings
	cargo fmt --all -- --check

build::
	cargo build

test::
	cargo build --all
	cargo test --all
	make -C trh test

ci:: check-enforce test
