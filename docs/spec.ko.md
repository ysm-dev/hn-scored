# hn-scored: 기술 명세서

Hacker News 스토리를 점수 기준으로 필터링한 RSS 피드.
사용자는 원하는 최소 점수를 선택하여 신호 대 잡음 비율을 조절한다.

---

## 0. 적합성

### 0.1 규범적 용어

`MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, `MAY`는
RFC 2119에 정의된 의미로 해석한다.

### 0.2 결정론적 계약

적합한 구현은 프로세스 시작 시 단 한 번의 `cycle_time`을
캡처해야 한다.

다음이 동일하다면:

- `cycle_time`
- CLI 인자
- 이전 `state.json` 바이트
- 이전에 영속화된 피드 출력 바이트
- upstream HTTP 응답

반드시 바이트 단위로 동일한 `state.json`, 96개의 피드 파일,
그리고 생성된 `_headers` 파일을 만들어야 한다.

`index.html`, 로깅, 레포지토리 구조, 테스트 전략, CI 워크플로우
세부사항은 이 문서에서 명시적으로 규범(normative)으로 선언하지
않는 한 정보성(informative)이다.

### 0.3 런타임 범위

3-7절과 14-18절이 규범적 런타임 계약을 정의한다.
정보성 섹션이 규범적 섹션과 충돌하면 규범적 섹션이 우선한다.

---

## 1. 목표 & 철학

### 1.1 최우선 목표

**결정론적 안정성이 모든 것에 우선한다.** discovery set에서
한 번 관측된 스토리는, 해당 스토리의 retained threshold crossing이
모두 만료되거나 성공적인 fetch가 `dead/deleted`를 확인하기 전까지
조용히 유실되지 않은 채 추적된다. 추적 중인 스토리에 대해서는
피드가 매분 갱신된다.

### 1.2 핵심 원칙

| 원칙 | 규칙 |
|------|------|
| **관측된 스토리 보존** | 한 번 발견된 스토리는 threshold crossing이 만료되거나 API가 `dead/deleted`를 확인할 때까지 state에 남고 피드 포함 자격을 유지한다. |
| **추적 중 스토리의 1분 신선도** | 각 cycle은 discovery endpoint와 현재 보존 중인 모든 story ID를 다시 fetch한다. Cache-Control: `max-age=60`. |
| **결정론적 출력** | 한 cycle에서 생성되는 모든 타임스탬프는 하나의 `cycle_time`에서 파생된다. 동일 입력은 동일 영속 바이트를 생성한다. |
| **우아한 실패 처리** | fetch 실패가 기존 state를 훼손해서는 안 된다. 기존에 추적 중이던 스토리는 후속 성공 fetch가 업데이트하거나 제거하기 전까지 그대로 유지된다. |

---

## 2. 개요

### 2.1 문제

Hacker News에는 점수 기준으로 RSS를 필터링하는 기본 기능이 없다.
고신호 스토리만 원하는 사용자는 모든 것을 훑어봐야 한다.

### 2.2 해결책

16개 점수 threshold에 대한 정적 RSS/Atom/JSON Feed 파일을 생성하여
1분마다 업데이트한다. Cloudflare Workers에서 1분 캐시로 호스팅한다.

### 2.3 URL 구조

```
https://hn.ysm.dev/feeds/article/100.xml       # RSS,  100점 이상, 원문 링크
https://hn.ysm.dev/feeds/comments/100.xml       # RSS,  100점 이상, HN 댓글 링크
https://hn.ysm.dev/feeds/article/100.atom       # Atom
https://hn.ysm.dev/feeds/article/100.json       # JSON Feed
```

---

## 3. 점수 Threshold

### 3.1 단계 (총 16개)

| 범위 | 간격 | 값 |
|------|------|----|
| 0 - 500 | 50 | 0, 50, 100, 150, 200, 250, 300, 350, 400, 450, 500 |
| 500 - 1000 | 100 | 600, 700, 800, 900, 1000 |

### 3.2 점수 정책: 한번 포함되면 유지

스토리의 점수가 처음으로 `score >= N`을 만족하면
`thresholds[N] = cycle_time`을 기록한다.

이 타임스탬프는 절대 덮어쓰지 않는다. 스토리는
`thresholds[N] >= cycle_time - 7 days`인 동안 threshold `N`에 대해
포함 자격을 유지한다. 이후 점수가 하락하더라도 해당 threshold 기록은
만료될 때까지 유지된다.

**예외**: 성공적인 fetch가 `dead == true` 또는 `deleted == true`를
반환하면, 해당 스토리는 모든 피드에서 즉시 제거하고 state에서도
즉시 삭제한다.

### 3.3 예상 볼륨

| Threshold | 일일 스토리 수 | 성격 |
|-----------|----------------|------|
| 0 | 300 - 500 | 전체 (firehose) |
| 50 | 40 - 80 | 인기 |
| 100 | 20 - 40 | 매우 인기 |
| 200 | 8 - 20 | 주요 스토리 |
| 300 | 3 - 10 | 예외적 |
| 500 | 0 - 3 | 바이럴 |
| 1000 | 0 - 1 | 희귀, 역사적 |

HN BigQuery 데이터 기준, 2024년 11월 - 2025년 11월.

---

## 4. 피드 명세

### 4.1 포맷 (3종)

| 포맷 | 확장자 | MIME 타입 |
|------|--------|-----------|
| RSS 2.0 | `.xml` | `application/rss+xml` |
| Atom 1.0 | `.atom` | `application/atom+xml` |
| JSON Feed 1.1 | `.json` | `application/feed+json` |

### 4.2 링크 타입 (2종)

| 타입 | 디렉토리 | `<link>` 대상 |
|------|----------|---------------|
| Article | `feeds/article/` | 원본 기사 URL |
| Comments | `feeds/comments/` | `https://news.ycombinator.com/item?id={id}` |

Self-post (Ask HN, Show HN, Launch HN)는 외부 URL이 없다.
article 피드는 이 경우 HN 댓글 URL로 fallback한다.

### 4.3 총 파일 수

16 threshold x 3 포맷 x 2 링크 타입 = **96개 파일** (cycle당)

### 4.4 RSS 전체 문서 예시

```xml
<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>Hacker News - 100+ points</title>
    <link>https://news.ycombinator.com</link>
    <description>Hacker News stories with 100 or more points</description>
    <lastBuildDate>Mon, 14 Apr 2025 12:34:56 +0000</lastBuildDate>
    <ttl>1</ttl>
    <generator>hn-scored</generator>
    <atom:link href="https://hn.ysm.dev/feeds/article/100.xml" rel="self" type="application/rss+xml"/>
    <item>
      <title>My YC app: Dropbox - Throw away your USB drive (3h 32m)</title>
      <link>http://www.getdropbox.com/u/2/screencast.html</link>
      <guid isPermaLink="false">https://news.ycombinator.com/item?id=8863</guid>
      <pubDate>Mon, 14 Apr 2025 12:34:56 +0000</pubDate>
      <description>423 points | 156 comments | getdropbox.com/u/2/screencast.html</description>
      <comments>https://news.ycombinator.com/item?id=8863</comments>
    </item>
  </channel>
</rss>
```

| 필드 | 값 |
|------|----|
| `<title>` | HN 제목 뒤에 해당 피드의 threshold에 도달하기까지 걸린 시간을 붙인다: `"{title} ({elapsed})"`. threshold 0("All Stories") 피드와 `story_time`을 알 수 없는 경우(`0`)에는 제목이 그대로 유지된다(접미사 없음). 정확한 알고리즘은 18.11 참고. |
| `<link>` | 원본 URL (article) 또는 HN 댓글 URL (comments 피드). |
| `<guid>` | `https://news.ycombinator.com/item?id={id}`. 모든 피드에서 동일. |
| `<pubDate>` | 해당 피드의 threshold를 처음 넘은 시각. |
| `<description>` | `{score} points \| {comments} comments \| {domain+path}` |
| `<comments>` | 항상 `https://news.ycombinator.com/item?id={id}`. |
| `<lastBuildDate>` | 렌더된 항목 중 가장 최신 `last_output_change_at`, 빈 피드면 `Thu, 01 Jan 1970 00:00:00 +0000`. |

description의 domain은 경로를 포함한다. 예: `github.com/foo/bar`.

### 4.5 채널 메타데이터

채널 메타데이터는 threshold와 링크 타입에 따라 다르다.

| 필드 | Article 피드 | Comments 피드 |
|------|-------------|---------------|
| `<title>` | `Hacker News - {N}+ points` | `Hacker News - {N}+ points (comments)` |
| `<link>` | `https://news.ycombinator.com` | `https://news.ycombinator.com` |
| `<description>` | `Hacker News stories with {N} or more points` | `Hacker News stories with {N} or more points (links to comments)` |
| `<atom:link rel="self">` | 이 피드의 자체 URL | 이 피드의 자체 URL |

**Threshold 0 특수 케이스**:
- Article title: `Hacker News - All Stories`
- Article description: `All Hacker News stories`
- Comments title: `Hacker News - All Stories (comments)`
- Comments description: `All Hacker News stories (links to comments)`

TTL: 1분.

### 4.6 피드 제한

- **보존 윈도우**: 스토리는 `thresholds[N]`가 존재하고
  `thresholds[N] >= cycle_time - 7 days`일 때만 threshold `N` 피드에
  포함 자격이 있다.
- **최대 항목 수**: 피드당 최대 200개의 eligible 스토리만 렌더한다.
  이것은 출력 상한일 뿐이며, state나 threshold 기록을 삭제하지는 않는다.
- **정렬 순서**: `thresholds[N]` 내림차순, 그다음 HN item ID 내림차순.
  정렬 후 200개 제한을 적용한다.
- **최신**이란 해당 피드 threshold를 처음 넘은 시각이며,
  HN 원래 게시 시각이 아니다.

### 4.7 스토리 필터링

- **포함**: `type == "story"` 만.
- **제외**: job, poll, pollopt, comment 타입.
- **제외**: `dead == true` 또는 `deleted == true` 스토리.

---

## 5. 데이터 파이프라인

### 5.1 데이터 소스

매 cycle의 fetch set은 Firebase discovery endpoint들과
`state.json`에 현재 보존 중인 모든 story ID로 구성된다.

| 소스 | 반환 | 목적 |
|------|------|------|
| `/v0/topstories.json` | 최대 500개 ID | 현재 높은 순위의 스토리 발견 |
| `/v0/beststories.json` | 최대 500개 ID | 며칠간 강세를 유지하는 스토리 발견 |
| `/v0/newstories.json` | 최대 500개 ID | 막 올라온 스토리를 초기에 발견 |
| 보존 중인 state ID | 최대 약 3,500개 ID | discovery endpoint에서 사라진 뒤에도 추적 중인 스토리의 score/comment를 최신으로 유지 |

중복 제거 후: cycle당 discovery ID는 약 800-1300개,
여기에 최대 약 3,500개의 retained ID가 추가된다.

**이유**: 세 discovery endpoint는 스토리 발견에 사용한다.
retained state ID는 매 cycle 다시 fetch하여, 이미 추적 중인 스토리가
topstories, beststories, newstories에서 사라진 이후에도 현재
score/comment 값을 유지하도록 한다.

### 5.2 API 전략

**주 API**: Firebase HN API만 사용.
- 실시간 점수 (인덱싱 지연 없음).
- 스토리 목록 fetch: `GET /v0/topstories.json`,
  `GET /v0/beststories.json`, `GET /v0/newstories.json`.
- 개별 항목 fetch: `GET /v0/item/{id}.json`.
- 동시성: 50개 동시 요청.
- 재시도: 스토리당 3회, 지수 백오프.
- 기존 추적 스토리의 item fetch가 실패하면, 이전 state를 그대로 유지하고
  다음 cycle에서 재시도한다.
- 새로 발견된 스토리의 item fetch가 실패하면 state entry를 만들지 않는다.
  해당 스토리가 다시 발견되면 다음 cycle에서 재시도한다.
- 세 discovery endpoint가 모두 재시도 후에도 실패하면,
  해당 cycle은 치명적(fatal)이며 바이너리는 exit code 1을 반환해야 한다.
- v1에는 Algolia fallback이 없다. 향후 degraded mode는 이후 스펙
  개정에서 추가될 수 있다.

### 5.3 처리 파이프라인

```
 1. 프로세스 시작 시 `cycle_time` 캡처
 2. state.json 로드
 3. 정리: `cycle_time - 7 days`보다 오래된 threshold 타임스탬프 제거,
    threshold가 하나도 남지 않은 스토리 제거
 4. `topstories` + `beststories` + `newstories` 병렬 fetch
 5. fetch set 구성 = 중복 제거된 discovery ID + retained state ID
 6. 스토리 상세 정보 fetch (50 동시, 각 3회 재시도)
 7. 성공적으로 받은 각 스토리 응답에 대해:
    a. `type` 또는 `title`이 누락/null이면 응답 무시;
       기존 state가 있으면 유지
    b. `type != "story"`이면 기존 state가 있으면 제거, 없으면 skip
    c. dead/deleted면 state에서 제거 후 skip
    d. 필드 정규화 후 state 업데이트
    e. 새로운 threshold crossing을 `cycle_time`으로 기록
    f. 영속 필드나 threshold map에 변경이 있으면
       `last_output_change_at = cycle_time`으로 설정
 8. 실패한 스토리 fetch에 대해서는 기존 state를 그대로 유지하고,
    새 entry는 만들지 않음
 9. 96개 피드 파일, `index.html`, `_headers`를 임시 출력 디렉토리에 생성
10. 다음 `state.json`, 피드 파일, `_headers`를 이전 영속 바이트와 비교
11. 이들 중 하나라도 바이트가 다르면 새 state를 쓰고,
    출력 디렉토리를 교체한 뒤 exit code 0 반환
12. 모두 바이트 단위로 동일하면 영속 파일을 바꾸지 않고
    exit code 2 반환
```

---

## 6. 상태 관리

### 6.1 저장소

`state.json`은 런타임에 레포지토리 루트에 위치한다. 이 파일은 추적 중인
스토리의 source of truth이며, 각 update run을 시작할 때 `state` GitHub
Release asset에서 복원된다. 바이트 내용이 바뀌면 workflow는 deploy 전에
해당 asset을 교체한다. 이 파일은 git에서 추적하지 않는다.

### 6.2 스키마

```json
{
  "version": 1,
  "last_output_change_at": "2025-04-14T15:00:56Z",
  "stories": {
    "8863": {
      "id": 8863,
      "title": "My YC app: Dropbox - Throw away your USB drive",
      "url": "http://www.getdropbox.com/u/2/screencast.html",
      "hn_url": "https://news.ycombinator.com/item?id=8863",
      "score": 423,
      "max_score": 450,
      "comments": 156,
      "by": "dhouston",
      "first_seen": "2025-04-14T12:34:56Z",
      "story_time": 1175714200,
      "last_output_change_at": "2025-04-14T15:00:56Z",
      "thresholds": {
        "0": "2025-04-14T12:34:56Z",
        "50": "2025-04-14T12:35:56Z",
        "100": "2025-04-14T12:40:56Z",
        "200": "2025-04-14T13:10:56Z",
        "300": "2025-04-14T14:00:56Z",
        "400": "2025-04-14T15:00:56Z"
      }
    }
  }
}
```

| 최상위 필드 | 설명 |
|-------------|------|
| `version` | 스키마 버전 |
| `last_output_change_at` | 영속 state가 마지막으로 변한 `cycle_time`. `stories`가 비어 있으면 `1970-01-01T00:00:00Z` 사용. |
| `stories` | 문자열 HN item ID -> story object 맵 |

| Story 필드 | 설명 |
|------------|------|
| `id` | HN item ID |
| `title` | 스토리 제목 (HTML 디코딩됨) |
| `url` | 원본 기사 URL (self-post는 빈 문자열) |
| `hn_url` | HN 댓글 페이지 URL |
| `score` | 현재 점수 |
| `max_score` | 지금까지 관측한 최고 점수 |
| `comments` | 현재 댓글 수 (`descendants`) |
| `by` | 작성자 |
| `first_seen` | 처음 추적된 시각 |
| `story_time` | 원래 HN 게시 타임스탬프 (Unix) |
| `last_output_change_at` | 이 story의 영속 필드가 마지막으로 변한 `cycle_time` |
| `thresholds` | threshold 값 -> ISO 8601 crossing 시각 맵 |

### 6.3 생애주기

1. **최초 발견**: 현재 넘은 모든 threshold를 `cycle_time`으로 기록한다.
   `first_seen`과 story `last_output_change_at`도 둘 다 `cycle_time`으로 설정한다.
2. **점수/댓글/제목/URL/작성자 변경**: 필드를 갱신하고
   story `last_output_change_at = cycle_time`으로 설정한다.
3. **점수 상승**: 필요하면 `max_score`를 갱신하고,
   새 threshold crossing을 `cycle_time`으로 기록한다.
4. **점수 하락**: 현재 `score`만 갱신한다. 기존 threshold 타임스탬프는 바뀌지 않는다.
5. **dead/deleted 또는 non-story**: entry를 완전히 제거한다.
6. **정리**: 7일이 지난 threshold 타임스탬프를 제거한다.
   threshold가 하나도 남지 않으면 story를 제거한다. `first_seen`은 만료 판단에 사용하지 않는다.
7. **최상위 타임스탬프**: `last_output_change_at`은 story별
   `last_output_change_at`의 최댓값이며, state가 비어 있으면 Unix epoch이다.

### 6.4 콜드 스타트

최초 실행 시 (빈 state), 유효한 discovered story는 모두 새로 추적된다.
현재 충족 중인 threshold는 모두 `cycle_time`으로 기록한다.

### 6.5 크기

최대 약 3,500개 항목 (7일 x 500/일). 항목당 약 250바이트.
총 약 900KB.

---

## 7. 인프라

### 7.1 호스팅: Cloudflare Workers

- **정적 자산**: 무료, 무제한 요청.
- **캐시**: `_headers` 파일로 1분.
- **도메인**: `hn.ysm.dev` (커스텀 도메인, CF-managed DNS).
- **정확한 generated header 규칙**: 18.7절 참조.

```
# generated dist/_headers
/feeds/article/*.xml
  Content-Type: application/rss+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/comments/*.xml
  Content-Type: application/rss+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/article/*.atom
  Content-Type: application/atom+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/comments/*.atom
  Content-Type: application/atom+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/article/*.json
  Content-Type: application/feed+json; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/comments/*.json
  Content-Type: application/feed+json; charset=utf-8
  Cache-Control: public, max-age=60

/index.html
  Cache-Control: public, max-age=60
```

### 7.2 배포 흐름

```
GitHub Actions (cron: */5 * * * *)
  |
  +-> GitHub Release에서 pre-built binary 다운로드
  +-> `state` GitHub Release에서 state.json 복원
  |
  +-> 최대 12분 동안 반복, 60초 간격:
  |     |
  |     +-> binary 실행 --state ./state.json --output ./dist/
  |     +-> Exit code 0 (state 또는 피드 출력 변경):
  |     |     +-> `state` release asset 교체
  |     |     +-> wrangler deploy
  |     +-> Exit code 1 (fatal):
  |     |     +-> 에러 로그, 루프 계속
  |     +-> Exit code 2 (영속 변경 없음):
  |     |     +-> Skip
  |     +-> 60초 대기 (마지막 반복 제외)
  |
  +-> 완료
```

`state.json`이 변경된 경우, 이것이 canonical source of truth이므로
release upload는 반드시 deploy보다 먼저 일어난다.

### 7.3 동시성 제어

```yaml
concurrency:
  group: update-feeds
  cancel-in-progress: false
```

새 run은 대기하며, 현재 run을 취소하지 않는다.

### 7.4 Secrets (GitHub Secrets)

| Secret | 용도 |
|--------|------|
| `CLOUDFLARE_API_TOKEN` | Wrangler 배포 인증 |
| `CLOUDFLARE_ACCOUNT_ID` | 대상 CF 계정 |

### 7.5 바이너리 빌드

`src/`/`Cargo.toml`/`Cargo.lock` 변경 시 별도 워크플로우:

1. Ubuntu에서 `x86_64-unknown-linux-gnu` 빌드.
2. 모든 테스트 + 커버리지 검사 실행.
3. GitHub Release에 바이너리 업로드.

Update 워크플로우는 pre-built binary를 다운로드한다.
update CI에서는 Rust를 컴파일하지 않는다.

### 7.6 장애 복구

성공한 cycle 이후 `wrangler deploy`가 실패한 경우:
- `state.json`이 바뀌었다면, `state` release에는 이미 canonical updated state가 들어 있다.
- `state.json`이 바뀌지 않았다면, 다음 cycle은 기존 release asset에서 동일한 출력을 다시 생성한다.
- deploy는 다음 성공적인 cycle에서 재시도된다.
- 자가 복구된다. 수동 개입은 필요 없다.

---

## 8. 레포지토리 구조

```
hn-scored/
├── .github/
│   └── workflows/
│       ├── build.yml
│       └── update.yml
├── src/
│   ├── main.rs            # 진입점, CLI 인자
│   ├── config.rs           # 상수 (threshold, 제한값, URL)
│   ├── api/
│   │   ├── mod.rs          # API 모듈
│   │   └── firebase.rs     # Firebase HN API 클라이언트
│   ├── state/
│   │   ├── mod.rs          # State 모듈
│   │   ├── store.rs        # state.json 로드/저장
│   │   ├── cleanup.rs      # 7일 만료, dead/deleted 제거
│   │   └── threshold.rs    # Threshold 교차 로직
│   ├── feed/
│   │   ├── mod.rs          # Feed 모듈
│   │   ├── rss.rs          # RSS 2.0 생성
│   │   ├── atom.rs         # Atom 1.0 생성
│   │   ├── json_feed.rs    # JSON Feed 1.1 생성
│   │   └── common.rs       # 공유 피드 유틸리티
│   ├── html/
│   │   ├── mod.rs          # HTML 모듈
│   │   └── index.rs        # index.html 생성
│   └── types.rs            # 공유 타입 (Story, State, Config)
├── tests/
│   ├── unit/
│   ├── integration/
│   └── e2e/
├── wrangler.jsonc
├── _headers
├── Cargo.toml
├── Cargo.lock
├── state.json               # Release에서 복원되는 런타임 파일 (gitignored)
├── LICENSE                 # MIT
├── README.md
└── docs/
    ├── spec.en.md
    └── spec.ko.md
```

**git에 포함되지 않음** (CF에 직접 배포):
```
dist/
├── feeds/
│   ├── article/
│   │   ├── 0.xml, 0.atom, 0.json
│   │   └── ... (16 threshold x 3 포맷)
│   └── comments/
│       ├── 0.xml, 0.atom, 0.json
│       └── ...
├── index.html
└── _headers
```

### 8.1 파일 크기 규칙

**`src/` 내 모든 파일은 200줄 이하**여야 한다. CI에서 강제한다.
모듈이 200줄을 넘으면 분할한다. 예외는 없다.

---

## 9. 랜딩 페이지

### 9.1 디자인

HN 오리지널 스타일:
- 오렌지 헤더 (`#ff6600`).
- 모노스페이스/시스템 폰트.
- 단순 HTML 테이블.

### 9.2 콘텐츠

- 정확한 HTML 마크업은 정보성이지 규범적이지 않다.
- 페이지는 반드시 16개 threshold를 오름차순으로 나열해야 한다.
- 각 threshold 행은 article/comments용 RSS, Atom, JSON Feed URL을 반드시 노출해야 한다.
- 제목: **hn-scored**
- 한 줄 설명: "Hacker News stories filtered by score. Pick a threshold and subscribe."
- 테이블: Threshold | Article (RSS / Atom / JSON) | Comments (RSS / Atom / JSON)
- URL별 복사 버튼.
- 푸터: `Last feed change: {state.last_output_change_at}` + GitHub 레포 링크.

푸터에 타임스탬프를 표시한다면, 현재 렌더 시각이 아니라
state-level `last_output_change_at`에서 파생되어야 한다.

---

## 10. 로깅

### 10.1 요약 (cycle당)

```
[2025-04-14T12:34:56Z] fetched=823 new=12 crossings=45 dead=2 changed=true duration=3.2s
```

### 10.2 경고

```
[WARN] fetch failed: item 12345 (attempt 3/3): connection timeout
```

---

## 11. 테스팅

### 11.1 규칙

- **Red/Green TDD**: 실패하는 테스트를 먼저 작성하고, 그 다음 통과시킨다.
- **커버리지 >= 90%**: CI에서 강제하며, 90% 미만이면 빌드가 실패한다.
- **모든 경로 테스트**: 정상, 엣지 케이스, 에러 경로를 모두 테스트한다.

### 11.2 단위 테스트

| 모듈 | 커버 대상 |
|------|-----------|
| `feed/rss.rs` | RSS 생성, 빈 피드, 최대 항목, HTML 엔티티, 특수 문자 |
| `feed/atom.rs` | Atom 생성, 동일 엣지 케이스 |
| `feed/json_feed.rs` | JSON Feed 생성, 동일 엣지 케이스 |
| `state/store.rs` | 로드, 저장, 결정론적 직렬화 순서, 손상된 state 복구 |
| `state/cleanup.rs` | 7일 threshold 만료, 빈 story 제거 |
| `state/threshold.rs` | crossing 감지, once-included 정책, 콜드 스타트, tie-break 정렬 |
| `api/firebase.rs` | list/item 파싱, 재시도, 부분 실패 처리 |
| `html/index.rs` | HTML 생성, threshold 순서, 푸터 타임스탬프 소싱 |

### 11.3 통합 테스트

모의 HTTP를 사용한 전체 파이프라인:
- 여러 cycle에 걸쳐 threshold를 통과하는 스토리.
- 늦게 threshold를 넘은 스토리가 crossing 시점부터 7일 유지되는지.
- 점수 하락 시 스토리가 그대로 유지되는지.
- dead/deleted 제거.
- 부분 네트워크 장애 시 기존 state가 유지되는지.
- discovery endpoint 전체 장애 시 fatal exit와 영속 변경 없음.
- 손상된 state.json 복구.
- 콜드 스타트 (빈 state).
- deterministic tie-break를 포함한 최대 200개 제한.
- unchanged cycle에서 state/feed 출력이 byte-identical인지.
- threshold 타임스탬프와 빈 story의 7일 정리.

### 11.4 E2E 테스트

- 실제 HN API를 사용한 단일 cycle 실행.
- 생성된 RSS/Atom/JSON output의 well-formed 검증.
- state.json round-trip 검증.

---

## 12. CI 설정

### 12.1 build.yml

트리거: `src/**`, `Cargo.toml`, `Cargo.lock` 변경 시.

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. 커버리지 검사 (>= 90%)
5. 줄 수 검사 (파일당 200줄 초과 금지)
6. `cargo build --release`
7. GitHub Release에 바이너리 업로드

### 12.2 update.yml

트리거: `schedule: */5 * * * *` 및 `workflow_dispatch`.

1. 레포 체크아웃.
2. Release에서 pre-built binary 다운로드.
3. wrangler 설치.
4. 5회 반복 루프 실행 (7.2절 참조).

---

## 13. Wrangler 설정

```jsonc
// wrangler.jsonc
{
  "name": "hn-scored",
  "compatibility_date": "2026-04-11",
  "workers_dev": true,
  "assets": {
    "directory": "./dist/"
  }
}
```

---

## 14. 정확한 포맷 명세

### 14.1 Atom 1.0 예시

```xml
<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Hacker News - 100+ points</title>
  <link href="https://news.ycombinator.com" rel="alternate"/>
  <link href="https://hn.ysm.dev/feeds/article/100.atom" rel="self"/>
  <id>https://hn.ysm.dev/feeds/article/100.atom</id>
  <updated>2025-04-14T12:45:56Z</updated>
  <subtitle>Hacker News stories with 100 or more points</subtitle>
  <generator>hn-scored</generator>
  <entry>
    <title>My YC app: Dropbox - Throw away your USB drive (3h 32m)</title>
    <link href="http://www.getdropbox.com/u/2/screencast.html" rel="alternate"/>
    <id>https://news.ycombinator.com/item?id=8863</id>
    <updated>2025-04-14T12:45:56Z</updated>
    <published>2025-04-14T12:40:56Z</published>
    <author><name>dhouston</name></author>
    <summary>423 points | 156 comments | getdropbox.com/u/2/screencast.html</summary>
  </entry>
</feed>
```

| Atom 필드 | 값 |
|-----------|----|
| `<feed><id>` | 피드 자체 URL. |
| `<feed><updated>` | 렌더된 엔트리 중 가장 최신 `last_output_change_at`, 빈 피드면 `1970-01-01T00:00:00Z`. RSS `<lastBuildDate>`와 같은 시각. |
| `<feed><link rel="self">` | 이 피드의 URL. |
| `<entry><id>` | RSS `<guid>`와 동일한 HN item URL. |
| `<entry><title>` | RSS `<title>`과 동일한 규칙 (4.4, 18.11 참고). |
| `<entry><updated>` | story `last_output_change_at`. |
| `<entry><published>` | 이 피드에 대한 threshold crossing 시각. |
| `<entry><author><name>` | HN 사용자명. `by`가 비어 있으면 `<author>` 자체를 생략한다. |
| `<entry><summary>` | RSS `<description>`과 동일한 평문. |

### 14.2 JSON Feed 1.1 예시

```json
{
  "version": "https://jsonfeed.org/version/1.1",
  "title": "Hacker News - 100+ points",
  "home_page_url": "https://news.ycombinator.com",
  "feed_url": "https://hn.ysm.dev/feeds/article/100.json",
  "description": "Hacker News stories with 100 or more points",
  "items": [
    {
      "id": "https://news.ycombinator.com/item?id=8863",
      "title": "My YC app: Dropbox - Throw away your USB drive (3h 32m)",
      "url": "http://www.getdropbox.com/u/2/screencast.html",
      "external_url": "https://news.ycombinator.com/item?id=8863",
      "content_text": "423 points | 156 comments | getdropbox.com/u/2/screencast.html",
      "date_published": "2025-04-14T12:40:56Z",
      "date_modified": "2025-04-14T12:45:56Z",
      "authors": [{"name": "dhouston"}]
    }
  ]
}
```

| JSON Feed 필드 | Article 피드 | Comments 피드 |
|----------------|-------------|---------------|
| `items[].id` | HN item URL | HN item URL |
| `items[].title` | RSS `<title>`과 동일한 규칙 (4.4, 18.11 참고). | 동일 |
| `items[].url` | 원본 기사 URL | HN 댓글 URL |
| `items[].external_url` | HN 댓글 URL | 원본 기사 URL (교차) |
| `items[].content_text` | 평문. RSS description과 동일. | 동일 |
| `items[].date_published` | ISO 8601 UTC. Threshold crossing 시각. | 동일 |
| `items[].date_modified` | ISO 8601 UTC. story `last_output_change_at`. | 동일 |
| `items[].authors` | `[{"name": "hn_username"}]`. `by`가 비어 있으면 필드를 생략한다. | 동일 |

참고: `url`과 `external_url`은 article/comments 피드 간에 **교차**된다.
Article 피드에서는 `url` = 원문, `external_url` = HN.
Comments 피드에서는 `url` = HN, `external_url` = 원문.
Self-post (원문 URL 없음)인 경우 둘 다 HN 댓글 URL을 가리킨다.

### 14.3 모든 타임스탬프는 UTC

- RSS: RFC 2822 형식. 예: `Mon, 14 Apr 2025 12:34:56 +0000`
- Atom: ISO 8601 / RFC 3339. 예: `2025-04-14T12:34:56Z`
- JSON Feed: ISO 8601 / RFC 3339. 예: `2025-04-14T12:34:56Z`
- state.json: ISO 8601. 예: `2025-04-14T12:34:56Z`

로컬 타임존은 사용하지 않는다. 항상 UTC이며 `Z` 접미사 또는 `+0000`을 사용한다.
한 cycle에서 새로 생성되는 모든 타임스탬프는 반드시 캡처된
`cycle_time`을 사용해야 한다.

### 14.4 텍스트 인코딩

- 모든 피드 콘텐츠는 **평문(plain text)**이며 HTML이 아니다.
- `<description>` / `<summary>` / `content_text`는 평문이다.
- HN API의 스토리 제목에는 HTML 엔티티(`&amp;`, `&#x27;`, `&lt;`)가
  포함될 수 있다. state와 피드에 저장하기 전에 반드시 **평문으로 디코딩**해야 한다.
- XML(RSS/Atom) 작성 시, 평문을 XML escape 처리한다
  (`&` -> `&amp;`, `<` -> `&lt;`, `>` -> `&gt;`, `"` -> `&quot;`).
- JSON Feed에서는 평문을 JSON escape한다 (`serde_json` 표준 동작).

---

## 15. 엣지 케이스 & 정확한 동작

### 15.1 Threshold 0 의미

`0` threshold 피드는 fetch set에 포함된 **모든 유효한 story**를 포함한다.
score가 0이거나 음수여도 포함된다. 유일한 필터는
`type == "story"`이고 dead/deleted가 아닌 것이다.

### 15.2 누락 또는 null API 필드

HN API 항목에는 필드가 누락될 수 있다. 정확한 처리 규칙은 다음과 같다.

| 필드 | 누락/null 시 |
|------|-------------|
| `score` | 0으로 처리. |
| `title` | 유효하지 않은 응답. 새 story는 만들지 않는다. story가 이미 있으면 기존 저장 값을 그대로 유지한다. |
| `url` | 빈 문자열로 처리 (self-post). |
| `type` | 유효하지 않은 응답. 새 story는 만들지 않는다. story가 이미 있으면 기존 저장 값을 그대로 유지한다. |
| `dead` | false로 처리. |
| `deleted` | false로 처리. |
| `descendants` | 0으로 처리 (댓글 수). |
| `by` | 빈 문자열 사용. |
| `time` | 0 (Unix epoch) 사용. |

성공한 응답에서 `type != "story"`이면, 해당 ID의 기존 state entry가 있다면 제거한다.
`type`이 누락/null인 경우는 삭제 신호가 아니라 invalid response이다.

### 15.3 Description용 URL 도메인 추출

URL에서 description 필드의 domain+path를 추출하는 규칙:

1. URL을 파싱한다. 파싱에 실패하면 원시 URL 문자열을 사용한다.
2. 스킴(`https://`, `http://`)을 제거한다.
3. `www.` 접두사가 있으면 제거한다.
4. 경로가 `/`만 있는 경우 후행 `/`를 제거한다.
5. 쿼리 스트링(`?...`)과 프래그먼트(`#...`)를 제거한다.
6. 비표준 포트(80/443 아닌 경우)는 유지한다.

예시:
```
https://www.github.com/foo/bar?ref=hn  -> github.com/foo/bar
https://blog.example.com:8080/post     -> blog.example.com:8080/post
https://example.com/                   -> example.com
https://example.com                    -> example.com
```

### 15.4 Self-post Description 도메인

Self-post (빈 URL)의 경우 description의 domain 필드에는
`news.ycombinator.com/item?id={id}`를 넣는다.

예: `42 points | 15 comments | news.ycombinator.com/item?id=12345`

### 15.5 "변경됨"의 정의 (Exit Code)

Exit code 0 (변경됨)은 이전에 영속화된 바이트와 비교했을 때
**다음 중 하나라도** 다르면 반환한다.

- 다음 `state.json` 바이트.
- 생성된 피드 파일 중 하나의 바이트.
- 생성된 `_headers` 바이트.
- 이전에 있어야 했던 필수 파일의 부재.

Exit code 2 (무변경)는 `state.json`, 모든 피드 파일, 생성된 `_headers`가
이전 영속 버전과 **바이트 단위로 동일**함을 의미한다.

`index.html`은 정보성이며, 적합성 판단에는 포함하지 않는다.

Exit code 1은 치명적 에러를 의미한다 (15.10 참조).

### 15.6 state.json 포맷 규칙

- **Pretty-print**: 2칸 들여쓰기.
- **UTF-8** 인코딩, BOM 없음.
- **LF 줄바꿈**을 사용하고 파일 끝에 trailing newline을 둔다.
- 최상위 키 순서는 정확히 `version`, `last_output_change_at`, `stories`.
- story는 문자열 ID (`"8863"`, `8863` 아님)를 키로 사용하며,
  numeric ID 오름차순으로 정렬한다.
- story object의 키 순서는 정확히 `id`, `title`, `url`, `hn_url`,
  `score`, `max_score`, `comments`, `by`, `first_seen`, `story_time`,
  `last_output_change_at`, `thresholds`.
- `thresholds`의 키는 문자열이며, numeric threshold 오름차순으로 정렬한다.

### 15.7 손상 또는 누락된 state.json

| 상태 | 동작 |
|------|------|
| 파일이 존재하지 않음 | 빈 state로 시작 (콜드 스타트). |
| 파일이 비어 있음 (0바이트) | 빈 state로 시작 (콜드 스타트). |
| 유효하지 않은 JSON | 에러 로그 후 빈 state로 시작. |
| 잘못된 `version` | 에러 로그 후 빈 state로 시작. |
| 유효하지만 일부 항목 형식 오류 | 잘못된 항목은 skip하고, 유효한 항목은 유지. |

잘못된 state 때문에 crash해서는 안 된다. 항상 우아하게 복구해야 한다.

### 15.8 재시도 백오프

개별 스토리 fetch에 대한 지수 백오프 + 지터:

| 시도 | 기본 지연 | 지터 포함 |
|------|----------|----------|
| 1 (최초) | 0ms | 0ms |
| 2 (1차 재시도) | 500ms | 250-750ms |
| 3 (2차 재시도) | 1000ms | 500-1500ms |

공식: `base_delay * 2^(attempt-2)`에 +/- 50% 랜덤 지터.
최대 지연은 2초로 제한한다.

### 15.9 Discovery Endpoint 실패

- 하나 또는 둘의 discovery endpoint가 재시도 후 실패하더라도,
  성공한 discovery list와 retained state ID로 cycle을 계속 진행한다.
- 세 discovery endpoint가 모두 재시도 후 실패하면 exit code 1을 반환하고,
  영속 state/output은 그대로 둔다.
- v1에는 secondary discovery provider가 없다.

### 15.10 Exit Code

| 코드 | 의미 | 셸에서의 동작 |
|------|------|--------------|
| 0 | `state.json`, 피드 파일, 또는 `_headers`가 변경됨. 영속 파일이 기록됨. | 필요하면 state 커밋 후 deploy. |
| 1 | 치명적 에러 (파일 쓰기 불가 등). | 에러 로그, 루프 계속. |
| 2 | 영속 변경 없음. | deploy skip. |

---

## 16. CLI 인터페이스

### 16.1 바이너리 이름

`hn-scored` (하이픈, crate/repo 이름과 일치)

### 16.2 인자

```
hn-scored --state <PATH> --output <PATH>

Options:
  --state <PATH>    state.json 경로 [기본값: ./state.json]
  --output <PATH>   피드 출력 디렉토리 [기본값: ./dist]
  --base-url <URL>  자기 참조 링크의 기본 URL
                    [기본값: https://hn.ysm.dev]
  --help            도움말 출력
  --version         버전 출력
```

다른 플래그는 없다. verbose/quiet 플래그도 없다. 로그 레벨은 고정이다.

`--base-url`은 절대 `http` 또는 `https` URL이어야 한다.
자기 참조 URL을 생성하기 전에, trailing slash가 있으면 정확히 하나만 제거한다.
정규화된 값은 RSS `<atom:link rel="self">`, Atom feed ID/self link,
JSON Feed `feed_url` 생성에 사용한다.

---

## 17. CI 운영 세부사항

### 17.1 런타임 상태 영속화

Update workflow는 rolling `state` release에서 `state.json`을 다운로드한다.
Exit code 0이면 deploy 전에 release asset을 교체한다. 일회성 migration
중에만 `state` release가 없고 추적 중인 `state.json`이 있으면 해당 파일로
release를 초기화한다.

### 17.2 GitHub Release 태그

`latest` release에는 pre-built binary가 들어 있다. Build workflow는 성공할
때마다 이 release를 **삭제 후 재생성**한다. 별도의 `state` release에는
rolling runtime state가 들어 있으며 build workflow가 삭제하지 않는다.

### 17.3 Wrangler 버전

워크플로우에서 고정:
```yaml
- run: npm install -g wrangler@4
```

메이저 버전은 고정하고, 마이너/패치는 유동적이다.

### 17.4 GitHub Actions 권한

```yaml
permissions:
  contents: write    # release 업로드용
```

### 17.5 E2E 테스트와 CI

실제 HN API를 호출하는 E2E 테스트는 **빌드 워크플로우에서 제외**한다.
`cargo test --ignored`로 수동 실행하거나 별도 nightly 워크플로우에서만 돌린다.
외부 API 불안정 때문에 release가 막히는 것을 방지하기 위함이다.

모의 HTTP를 사용하는 통합 테스트는 빌드 워크플로우에서 실행한다.

---

## 18. 나머지 정확한 동작

### 18.1 Description 숫자 포맷

천 단위 구분자는 사용하지 않는다. 순수 정수만 사용한다.
- 올바름: `1234 points | 567 comments`
- 틀림: `1,234 points | 567 comments`

### 18.2 빈 피드

threshold에 해당하는 스토리가 0개인 경우
(예: threshold 1000에서 최근 7일간 crossing이 없음),
**빈 피드 파일도 반드시 생성**한다.
채널/피드 메타데이터는 포함하지만 item/entry는 0개다.
파일 생성을 생략해서는 안 된다.

빈 피드의 feed-level 타임스탬프는 Unix epoch를 사용한다.
- RSS `<lastBuildDate>` = `Thu, 01 Jan 1970 00:00:00 +0000`
- Atom `<feed><updated>` = `1970-01-01T00:00:00Z`

### 18.3 출력 디렉토리 처리

1. 각 cycle은 전체 출력을 임시 디렉토리에 생성한다.
2. 필요한 디렉토리 구조는 `dist/feeds/article/`, `dist/feeds/comments/`이다.
3. 필요한 파일은 96개 피드 파일, `dist/index.html`, `dist/_headers`다.
   최종 디렉토리에는 추가 파일이 있으면 안 된다.
4. 바이너리는 부분적으로만 작성된 `dist/` 디렉토리를 남겨서는 안 된다.
   전체 생성이 성공한 뒤에만 출력 디렉토리를 교체한다.
5. 정확한 swap 메커니즘은 구현체에 맡기지만, 최종 on-disk 결과는
   reader 관점에서 atomic해야 한다.
6. `_headers` 파일은 바이너리가 직접 생성하며, 레포 루트에서 복사하지 않는다.

### 18.4 첫 실행 시 변경 감지

첫 실행에서 이전의 `state.json`, 피드 출력, 생성된 `_headers`가 없다면,
해당 cycle은 "변경됨"으로 간주한다.
Exit code 0을 반환한다.

필수 영속 파일 중 하나라도 없으면, 바이트 비교 단계에서 차이로 간주한다.

### 18.5 200줄 제한 적용

- 계산 기준: `wc -l` (전체 줄바꿈 문자 수).
- **포함**: 빈 줄, 주석, doc 주석, attribute.
- **적용 대상**: `src/` 아래 모든 `*.rs` 파일.
- **적용하지 않음**: `tests/`, `build.rs`, `Cargo.toml`.
- CI 검사 명령:
  ```bash
  find src -name '*.rs' | while read f; do
    lines=$(wc -l < "$f")
    if [ "$lines" -gt 200 ]; then
      echo "FAIL: $f has $lines lines (max 200)" && exit 1
    fi
  done
  ```

### 18.6 `--version` 출력

`Cargo.toml`의 버전을 출력한다:
```
hn-scored 0.1.0
```

### 18.7 `_headers` 내용 (바이너리가 생성)

```
/feeds/article/*.xml
  Content-Type: application/rss+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/comments/*.xml
  Content-Type: application/rss+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/article/*.atom
  Content-Type: application/atom+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/comments/*.atom
  Content-Type: application/atom+xml; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/article/*.json
  Content-Type: application/feed+json; charset=utf-8
  Cache-Control: public, max-age=60

/feeds/comments/*.json
  Content-Type: application/feed+json; charset=utf-8
  Cache-Control: public, max-age=60

/index.html
  Cache-Control: public, max-age=60
```

이렇게 하면 Cloudflare Workers가 `.atom`처럼 표준 MIME 매핑이 없는
확장자까지 포함해 올바른 MIME 타입을 서빙할 수 있다.

### 18.8 Comments 피드의 RSS Channel Link

`<channel><link>`는 article과 comments 피드 모두에서 항상
`https://news.ycombinator.com`이다.
이 값은 피드 자체가 아니라, 피드가 다루는 웹사이트를 나타낸다.
피드의 self-link는 `<atom:link rel="self">`에 있다.

### 18.9 workers.dev URL

`wrangler.jsonc`의 `workers_dev` 설정은 `true`이다. 두 URL 모두 동작한다.
- `https://hn.ysm.dev/...` (커스텀 도메인)
- `https://hn-scored.{subdomain}.workers.dev/...` (기본값)

### 18.10 랜딩 페이지 복사 버튼

JavaScript Clipboard API (`navigator.clipboard.writeText()`)를 사용한다.
HTML 내 인라인 `<script>`를 사용하며, 외부 JS 의존성은 없다.
JS가 비활성화되어도 URL은 평문으로 보이므로 수동 복사가 가능하다.
기능 손실은 없고 버튼만 비활성화된다.

### 18.11 제목 경과 시간 접미사

RSS `<title>`, Atom `<entry><title>`, JSON Feed `items[].title` 모두
스토리가 게시된 뒤 **해당 피드의 threshold**에 도달하기까지 걸린 시간을
덧붙인다.

**계산식**:

```
total_minutes = floor((thresholds[N] - story_time)을 분 단위로), 0 이상으로 clamp
days    = total_minutes / (24 * 60)
hours   = (total_minutes % (24 * 60)) / 60
minutes = total_minutes % 60
```

여기서 `N`은 렌더링 중인 피드의 threshold이다 (`thresholds[N]`은 해당
항목의 `<pubDate>` / `<published>` / `date_published`에 이미 사용되는
시각과 동일하다).

**렌더링 규칙**:
- 접미사는 `"{days}d {hours}h {minutes}m"` 형식으로 만든다.
- `days == 0`이면 `d` 구간은 완전히 생략한다.
- `hours == 0`이면 `h` 구간은 완전히 생략한다.
- `m` 구간은 값이 `0`이어도 **항상** 표시한다.
- 최종 제목: `"{title} ({suffix})"`.

예시: `(2d 3h 32m)`, `(3h 32m)`, `(32m)`, `(1d 0m)`, `(1h 0m)`.

**예외** (아래 경우에는 원본 HN 제목을 그대로 사용하고 접미사를 붙이지
않는다):
- Threshold 0 ("All Stories") 피드. Threshold 0은 최초 발견 시점에
  즉시 충족되므로, 모든 항목에서 거의 0에 가까운 잡음이 된다.
- `story_time == 0`인 스토리 (15.2의 "누락된 `time` 필드" 폴백). Unix
  epoch를 기준으로 경과 시간을 계산하면 의미 없는 수십 년 단위 값이
  나온다.

이 접미사는 한번 기록되면 값이 바뀌지 않는다: `thresholds[N]`은
write-once이며 (3.2), 정상 동작 하에서 스토리가 처음 추적된 이후
`story_time`도 바뀌지 않는다. 따라서 실시간으로 바뀌는 원본 점수와
달리, 이 값은 feed reader가 캐시할 수 있는 제목에 안전하게 포함할 수
있다.

---

## 19. 라이선스

MIT License.
