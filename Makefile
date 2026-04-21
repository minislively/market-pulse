.PHONY: test smoke smoke-brave-adapter smoke-brave-live fmt install

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
	MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/search-command/fixture-jsonl {query}' cargo run --quiet --bin mp -- "달러 강세가 코스피에 부담임?" --research --no-save
	$(MAKE) smoke-brave-adapter
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- think "금리가 부담인데도 반도체가 버티는 것 같다" --no-save

smoke-brave-adapter:
	unset BRAVE_SEARCH_API_KEY; if ./adapters/search-command/brave-jsonl "달러 강세가 코스피에 부담임?" >/tmp/market-pulse-brave.out 2>/tmp/market-pulse-brave.err; then echo "expected missing-key failure"; exit 1; fi
	test ! -s /tmp/market-pulse-brave.out
	grep -q "BRAVE_SEARCH_API_KEY is not set" /tmp/market-pulse-brave.err
	./adapters/search-command/brave-jsonl --fixture "달러 강세가 코스피에 부담임?"
	MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl --fixture {query}' cargo run --quiet --bin mp -- "달러 강세가 코스피에 부담임?" --research --no-save

smoke-brave-live:
	@if [ -n "$${BRAVE_SEARCH_API_KEY:-}" ]; then \
		MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl {query}' cargo run --quiet --bin mp -- "달러 강세가 코스피에 부담임?" --research --no-save; \
	else \
		echo "Skipping live Brave smoke: BRAVE_SEARCH_API_KEY is not set"; \
	fi

install:
	cargo install --path . --force
