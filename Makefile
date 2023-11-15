check::
	cargo clippy --all
	cargo fmt --all

check-enforce::
	cargo clippy --all
	cargo fmt --all -- --check

build::
	cargo build

test::
	cargo test --all

ci:: check-enforce test
