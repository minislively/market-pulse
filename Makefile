.PHONY: test smoke

test:
	python3.11 -m unittest discover -s tests -v

smoke:
	MARKET_PULSE_HOME=$$(mktemp -d) python3.11 -m market_pulse now --compact
	MARKET_PULSE_HOME=$$(mktemp -d) python3.11 -m market_pulse think "금리가 부담인데도 반도체가 버티는 것 같다" --no-save
