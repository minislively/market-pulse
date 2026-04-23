---
name: mp
description: market-pulse CLI를 실행해 터미널 기반 시장 질문 탐색, 리서치 스캐폴드, 시장 카드, Daily Radar/FOMO 체크포인트, 주간 학습 카드, 날짜 창 확인, 로컬 저널 recall 검색, 사용자 해석 피드백, 리뷰를 제공합니다. /mp 질문, /mp ask 질문, /mp research 질문, /mp now, /mp watch, /mp fomo, /mp week, /mp calendar, /mp regime, /mp think, /mp review, /mp find 또는 오늘 시황/시장 펄스/이번주 복기/전에 금리 찾아줘/NVDA 리서치/포모/시장 생각 피드백 요청에 사용합니다.
---

# market-pulse Claude Command

`market-pulse`의 로컬 `mp` Rust CLI를 호출하는 얇은 Claude Code slash command입니다.

## Usage

```text
/mp 금리가 내려간 게 진짜 완화 기대 때문임?
/mp ask 대형 IPO 때문에 성장주가 강한 걸까?
/mp research 금리 하락이 성장주에 좋은 신호임?
/mp 오늘 시황
/mp NVDA
/mp NVDA 리서치
/mp 전에 금리 찾아줘 --limit 3
/mp 이번주 복기
/mp now
/mp watch
/mp fomo
/mp week
/mp calendar
/mp regime
/mp think 금리가 부담인데도 반도체가 버티는 것 같다
/mp review
/mp review --date 2026-04-21
/mp review --days 1
/mp review --this-week
/mp find 금리 --this-week
```

인자가 없으면 `/mp now`로 처리합니다.

## Instructions

1. 사용자의 인자를 파악합니다.
   - 없음 또는 `now`: `mp now` 실행
   - `watch`: `mp watch` 실행
   - `fomo`: `mp fomo` 실행
   - `week`: `mp week` 실행
   - `calendar` 또는 `cal`: `mp calendar` 실행
   - `regime`: `mp regime` 실행
   - `ask <question>`: `mp ask "<question>"` 실행
   - `research <question>`: `mp research "<question>"` 실행
   - 알려진 명령어가 아닌 일반 텍스트: 그대로 `mp`에 전달해 CLI의 deterministic natural alias router가 처리하게 둠
     - 예: `오늘 시황`, `시장 펄스`: 현재 시장 카드
     - 예: `NVDA`, `비트코인`: safe inquiry scaffold
     - 예: `NVDA 리서치`, `반도체 왜 오름?`: research intent routing
     - 예: `오늘 복기`, `지난주 리뷰`: review selector routing
     - 예: `전에 금리 찾아줘`: find routing
     - 예: `내 생각 ...`, `메모 ...`: think routing
   - `think <text>`: `mp think "<text>"` 실행
   - `review`: `mp review` 실행
   - `review --date YYYY-MM-DD`: `mp review --date YYYY-MM-DD` 실행
   - `review --days N`: `mp review --days N` 실행
   - `review --today|--yesterday|--this-week|--last-week`: 같은 selector로 `mp review` 실행
   - `find <query>` 또는 `search <query>`: `mp find "<query>"` 실행, selector가 있으면 보존
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
- `watch` / `fomo`는 terminal-only Daily Radar와 FOMO decision-hygiene 체크포인트로 유지하고, 외부 알림/메신저/거래 지시를 프롬프트에서 추가하지 않습니다.
- research 모드에서는 source metadata/no-provider fallback과 source-backed vs inference 구분을 유지합니다.
- `MARKET_PULSE_SEARCH_CMD`가 설정되어 있으면 로컬 CLI가 외부 source metadata를 받아오며, command adapter는 그 경계를 보존합니다.
- 자연어/티커 처리는 CLI의 작은 deterministic alias layer로만 취급하고, slash command 프롬프트에서 별도 분류기를 만들지 않습니다.
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

### `/mp watch`

```bash
mp watch
```

Daily Radar: 오늘 볼 scenario/watch/confirm/falsify와 FOMO 질문을 터미널에 출력하고, `--no-save`가 없으면 `radar` JSONL 이벤트로 저장합니다.

### `/mp fomo`

```bash
mp fomo
```

FOMO Checkpoint: 최신 radar/pulse 문맥을 가져와 “증거인지, 기회비용 공포인지”를 분리하는 pause card를 출력하고, `--no-save`가 없으면 `fomo_check` JSONL 이벤트로 저장합니다.

### `/mp week`

```bash
mp week
```

### `/mp calendar`

```bash
mp calendar
```

로컬 리뷰 날짜창과 함께 US equities(NYSE/Nasdaq) / Korea equities(KRX/KOSPI)의 static regular-session 문맥, `mp now` / `mp week` close-basis 해석 브릿지를 보여줍니다. 첫 버전은 full holiday/early-close DB나 live event calendar가 아니라는 경계를 유지합니다.

### `/mp regime`

```bash
mp regime
```

### `/mp think ...`

```bash
mp think "금리가 부담인데도 반도체가 버티는 것 같다"
```

### `/mp review`

```bash
mp review
mp review --date 2026-04-21
mp review --days 1
mp review --this-week
```

### `/mp find ...query...`

```bash
mp find "금리" --this-week
```
