.PHONY: test smoke fmt install

fmt:
	cargo fmt --check

test:
	cargo test

smoke:
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- now --compact
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- ask "금리가 내려간 게 완화 기대 때문임?" --no-save
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- "대형 IPO 때문에 성장주가 강한 걸까?" --no-save
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- research "금리 하락이 성장주에 좋은 신호임?" --no-save
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- "대형 IPO 때문에 성장주가 강한 걸까?" --research --no-save
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- think "금리가 부담인데도 반도체가 버티는 것 같다" --no-save

install:
	cargo install --path . --force
