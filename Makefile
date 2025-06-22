.PHONY: lint server test copyconfig

lint:
	cargo fmt
	cargo clippy -- -D warnings

server:
	RUST_BACKTRACE=1 RUST_LOG=debug cargo run -- server

test:
ifdef TEST
	RUST_BACKTRACE=1 cargo test $(TEST) --verbose -- --nocapture
else
	RUST_BACKTRACE=1 cargo test --verbose -- --nocapture
endif

copyconfig:
	cp ../main-checkout/ayb.toml .
