.PHONY: test smoke smoke-brave-adapter smoke-brave-live smoke-earnings smoke-earnings-live fmt install

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
	$(MAKE) smoke-earnings
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- think "금리가 부담인데도 반도체가 버티는 것 같다" --no-save

smoke-brave-adapter:
	unset BRAVE_SEARCH_API_KEY; if ./adapters/search-command/brave-jsonl "달러 강세가 코스피에 부담임?" >/tmp/market-pulse-brave.out 2>/tmp/market-pulse-brave.err; then echo "expected missing-key failure"; exit 1; fi
	test ! -s /tmp/market-pulse-brave.out
	grep -q "BRAVE_SEARCH_API_KEY is not set" /tmp/market-pulse-brave.err
	./adapters/search-command/brave-jsonl --fixture "달러 강세가 코스피에 부담임?"
	MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl --fixture {query}' cargo run --quiet --bin mp -- "달러 강세가 코스피에 부담임?" --research --no-save

smoke-earnings:
	MARKET_PULSE_HOME=$$(mktemp -d) cargo run --quiet --bin mp -- earnings --no-save
	./adapters/earnings-command/fixture-jsonl "recent major US earnings results EPS revenue guidance stock reaction source"
	MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/earnings-command/fixture-jsonl {query}' cargo run --quiet --bin mp -- earnings --no-save
	./adapters/earnings-command/yahoo-jsonl --fixture "upcoming major US earnings this week next week calendar radar source"
	MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/earnings-command/yahoo-jsonl --fixture {query}' cargo run --quiet --bin mp -- earnings --no-save

smoke-earnings-live:
	@if [ "$${MARKET_PULSE_LIVE_EARNINGS:-}" = "1" ]; then \
		if ./adapters/earnings-command/yahoo-jsonl "upcoming major US earnings this week next week calendar radar source" >/tmp/market-pulse-earnings-live.out 2>/tmp/market-pulse-earnings-live.err; then \
			MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/earnings-command/yahoo-jsonl {query}' cargo run --quiet --bin mp -- earnings --no-save; \
		else \
			status=$$?; \
			if [ "$$status" = "2" ]; then \
				echo "Skipping live earnings smoke: Yahoo adapter unavailable or source shape changed"; \
				cat /tmp/market-pulse-earnings-live.err; \
			else \
				cat /tmp/market-pulse-earnings-live.err; exit $$status; \
			fi; \
		fi; \
	else \
		echo "Skipping live earnings smoke: set MARKET_PULSE_LIVE_EARNINGS=1 to enable"; \
	fi

smoke-brave-live:
	@if [ -n "$${BRAVE_SEARCH_API_KEY:-}" ]; then \
		MARKET_PULSE_HOME=$$(mktemp -d) MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl {query}' cargo run --quiet --bin mp -- "달러 강세가 코스피에 부담임?" --research --no-save; \
	else \
		echo "Skipping live Brave smoke: BRAVE_SEARCH_API_KEY is not set"; \
	fi

install:
	cargo install --path . --force
