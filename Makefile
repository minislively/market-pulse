.PHONY: test smoke fmt install

fmt:
	cargo fmt --check

test:
	cargo test

smoke:
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- now --compact
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- think "금리가 부담인데도 반도체가 버티는 것 같다" --no-save

install:
	cargo install --path . --force
