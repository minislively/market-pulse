---
name: mp
description: market-pulse CLI를 실행해 터미널 기반 시장 질문 탐색, 리서치 스캐폴드, 시장 카드, 사용자 해석 피드백, 리뷰를 제공합니다. /mp 질문, /mp ask 질문, /mp research 질문, /mp now, /mp think, /mp review 또는 오늘 시황/시장 펄스/시장 생각 피드백 요청에 사용합니다.
---

# market-pulse Claude Command

`market-pulse`의 로컬 `mp` Rust CLI를 호출하는 얇은 Claude Code slash command입니다.

## Usage

```text
/mp 금리가 내려간 게 진짜 완화 기대 때문임?
/mp ask 대형 IPO 때문에 성장주가 강한 걸까?
/mp research 금리 하락이 성장주에 좋은 신호임?
/mp now
/mp think 금리가 부담인데도 반도체가 버티는 것 같다
/mp review
```

인자가 없으면 `/mp now`로 처리합니다.

## Instructions

1. 사용자의 인자를 파악합니다.
   - 없음 또는 `now`: `mp now` 실행
   - `ask <question>`: `mp ask "<question>"` 실행
   - `research <question>`: `mp research "<question>"` 실행
   - 알려진 명령어가 아닌 일반 텍스트: `mp "<question>"` 실행
   - `think <text>`: `mp think "<text>"` 실행
   - `review`: `mp review` 실행
2. 가능한 한 로컬 CLI를 직접 실행합니다.
3. CLI 출력은 핵심만 정리하되, 중요한 피드백 구조는 유지합니다.
4. `MARKET_PULSE_SEARCH_CMD`가 설정되어 있으면 `mp`가 제한형 JSONL source bridge로 사용하게 두고, slash command 프롬프트에서 검색을 재구현하지 않습니다.
5. `mp` 실행 파일이 없으면 아래 설치 안내를 제공합니다.

```bash
cd ~/dev/market-pulse
cargo install --path . --force
```

## Safety / Product Boundary

- 매수/매도 추천, 목표가, 손절가, 포트폴리오 지시는 하지 않습니다.
- 투자 조언이 아니라 시장 문해력과 사고 훈련으로 프레이밍합니다.
- 선호 구조: question breakdown, possible explanations, evidence to check, counter-view, next better question.
- research 모드에서는 source metadata/no-provider fallback과 source-backed vs inference 구분을 유지합니다.
- `MARKET_PULSE_SEARCH_CMD`가 설정되어 있으면 로컬 CLI가 외부 source metadata를 받아오며, command adapter는 그 경계를 보존합니다.
- 피할 것: 자극적 헤드라인, 숫자만 나열, 단일 정답처럼 말하기.

## Examples

### `/mp ...question...`

```bash
mp "금리가 내려간 게 진짜 완화 기대 때문임?"
```

### `/mp ask ...question...`

```bash
mp ask "대형 IPO 때문에 성장주가 강한 걸까?"
```

### `/mp research ...question...`

```bash
mp research "금리 하락이 성장주에 좋은 신호임?"
```

### `/mp now`

```bash
mp now
```

### `/mp think ...`

```bash
mp think "금리가 부담인데도 반도체가 버티는 것 같다"
```

### `/mp review`

```bash
mp review
```
