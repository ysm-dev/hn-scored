# hn-scored: Technical Specification

Hacker News RSS feeds filtered by score threshold.
Users subscribe to their preferred minimum score to control signal-to-noise ratio.

---

## 0. Conformance

### 0.1 Normative Language

The key words `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, and `MAY`
are to be interpreted as described in RFC 2119.

### 0.2 Deterministic Contract

A conforming implementation MUST capture a single `cycle_time` at
process start.

Given the same:

- `cycle_time`
- CLI arguments
- previous `state.json` bytes
- previous persisted feed output bytes
- upstream HTTP responses

it MUST produce byte-identical `state.json`, the 96 feed files,
and the generated `_headers` file.

`index.html`, logging, repository layout, testing strategy, and CI
workflow details are informative unless this document explicitly marks
them normative.

### 0.3 Runtime Scope

Sections 3-7 and 14-18 define the normative runtime contract.
If an informative section conflicts with a normative section, the
normative section wins.

---

## 1. Goals & Philosophy

### 1.1 Primary Goal

**Deterministic reliability above all else.** Once a story is observed
from the discovery set, it is tracked without silent loss until all of
its retained threshold crossings expire or a successful fetch marks it
dead/deleted. Feeds update every minute for tracked stories.

### 1.2 Core Principles

| Principle | Rule |
|-----------|------|
| **Observed-story retention** | Once discovered, a story stays in state and remains feed-eligible until its threshold crossings age out or the API confirms `dead/deleted`. |
| **1-minute freshness for tracked stories** | Each cycle re-fetches the discovery endpoints and every currently retained story ID. Cache-Control: `max-age=60`. |
| **Deterministic output** | All timestamps created during a cycle derive from the single captured `cycle_time`. Same inputs produce identical persisted bytes. |
| **Graceful failure** | Failed fetches never corrupt prior state. Existing tracked stories stay unchanged until a later successful fetch updates or removes them. |

---

## 2. Overview

### 2.1 Problem

Hacker News has no built-in way to filter stories by score via RSS.
Users who only care about high-signal stories must scroll through everything.

### 2.2 Solution

Generate static RSS/Atom/JSON Feed files for 16 score thresholds,
updated every minute. Hosted on Cloudflare Workers with 1-minute cache.

### 2.3 URL Structure

```
https://hn.ysm.dev/feeds/article/100.xml       # RSS,  100+ points, article links
https://hn.ysm.dev/feeds/comments/100.xml       # RSS,  100+ points, HN comment links
https://hn.ysm.dev/feeds/article/100.atom       # Atom
https://hn.ysm.dev/feeds/article/100.json       # JSON Feed
```

---

## 3. Score Thresholds

### 3.1 Tiers (16 total)

| Range | Interval | Values |
|-------|----------|--------|
| 0 - 500 | 50 | 0, 50, 100, 150, 200, 250, 300, 350, 400, 450, 500 |
| 500 - 1000 | 100 | 600, 700, 800, 900, 1000 |

### 3.2 Score Policy: Once Included, Always Included

When a story's score first satisfies `score >= N`, record
`thresholds[N] = cycle_time`.

That timestamp is never overwritten. The story remains eligible for
threshold `N` while `thresholds[N] >= cycle_time - 7 days`.
If the score later drops (e.g., downvotes), the threshold record remains
until it expires.

**Exception**: If a successful fetch reports `dead == true` or
`deleted == true`, remove the story from all feeds and purge it from
the active `stories` map immediately. Preserve its `max_scores` history.

### 3.3 Approximate Volume

| Threshold | Stories/day | Character |
|-----------|-------------|-----------|
| 0 | 300 - 500 | Firehose |
| 50 | 40 - 80 | Popular |
| 100 | 20 - 40 | Very popular |
| 200 | 8 - 20 | Major stories |
| 300 | 3 - 10 | Exceptional |
| 500 | 0 - 3 | Viral |
| 1000 | 0 - 1 | Rare, historic |

Based on HN BigQuery data, Nov 2024 - Nov 2025.

---

## 4. Feed Specification

### 4.1 Formats (3)

| Format | Extension | MIME Type |
|--------|-----------|-----------|
| RSS 2.0 | `.xml` | `application/rss+xml` |
| Atom 1.0 | `.atom` | `application/atom+xml` |
| JSON Feed 1.1 | `.json` | `application/feed+json` |

### 4.2 Link Types (2)

| Type | Directory | `<link>` Target |
|------|-----------|-----------------|
| Article | `feeds/article/` | Original article URL |
| Comments | `feeds/comments/` | `https://news.ycombinator.com/item?id={id}` |

Self-posts (Ask HN, Show HN, Launch HN) have no external URL.
Article feed falls back to the HN comments URL for these.

### 4.3 Total Files

16 thresholds x 3 formats x 2 link types = **96 files** per cycle.

### 4.4 RSS Full Document Example

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

| Field | Value |
|-------|-------|
| `<title>` | HN title, plus an elapsed-time suffix showing how long the story took to reach this feed's threshold after being posted: `"{title} ({elapsed})"`. Omitted (title is unchanged) for the threshold-0 "All Stories" feed and when `story_time` is unknown (`0`). Exact algorithm: see 18.11. |
| `<link>` | Original URL (article) or HN comments (comments feed). |
| `<guid>` | `https://news.ycombinator.com/item?id={id}`. Same across all feeds. |
| `<pubDate>` | Timestamp when the story first crossed this feed's threshold. |
| `<description>` | `{score} points \| {comments} comments \| {domain+path}` |
| `<comments>` | Always `https://news.ycombinator.com/item?id={id}`. |
| `<lastBuildDate>` | Latest `last_output_change_at` among rendered items, or `Thu, 01 Jan 1970 00:00:00 +0000` if the feed is empty. |

Domain in description includes the path: e.g., `github.com/foo/bar`.

### 4.5 Channel Metadata

Channel metadata varies by threshold and link type:

| Field | Article feed | Comments feed |
|-------|-------------|---------------|
| `<title>` | `Hacker News - {N}+ points` | `Hacker News - {N}+ points (comments)` |
| `<link>` | `https://news.ycombinator.com` | `https://news.ycombinator.com` |
| `<description>` | `Hacker News stories with {N} or more points` | `Hacker News stories with {N} or more points (links to comments)` |
| `<atom:link rel="self">` | Self URL of this feed | Self URL of this feed |

**Threshold 0 special cases**:
- Article title: `Hacker News - All Stories`
- Article description: `All Hacker News stories`
- Comments title: `Hacker News - All Stories (comments)`
- Comments description: `All Hacker News stories (links to comments)`

TTL: 1 minute.

### 4.6 Feed Limits

- **Retention window**: A story is eligible for threshold `N` only if
  `thresholds[N]` exists and `thresholds[N] >= cycle_time - 7 days`.
- **Max items**: Render at most 200 eligible stories per feed.
  This is an output cap only; it does not delete state or threshold
  records by itself.
- **Sort order**: `thresholds[N]` descending, then HN item ID descending.
  Apply the sort before enforcing the 200-item cap.
- **Newest** means the time the story first exceeded the feed's
  threshold, not the story's HN submission time.

### 4.7 Story Filtering

- **Included**: `type == "story"` only.
- **Excluded**: job, poll, pollopt, comment types.
- **Excluded**: Stories with `dead == true` or `deleted == true`.

---

## 5. Data Pipeline

### 5.1 Data Sources

The per-cycle fetch set is built from the Firebase discovery endpoints
plus all currently retained story IDs from `state.json`.

| Source | Returns | Purpose |
|--------|---------|---------|
| `/v0/topstories.json` | Up to 500 IDs | Discover stories currently ranked highly |
| `/v0/beststories.json` | Up to 500 IDs | Discover stories that remain strong over multiple days |
| `/v0/newstories.json` | Up to 500 IDs | Discover fresh stories before they rise |
| Retained state IDs | Up to ~3,500 IDs | Keep already-tracked stories fresh even after they leave discovery endpoints |

After deduplication: ~800-1300 discovery IDs per cycle,
plus up to ~3,500 retained IDs.

**Rationale**: The three discovery endpoints are used to find stories.
Retained state IDs are re-fetched every cycle so tracked stories keep
their current score/comment values even after they drop out of
topstories, beststories, or newstories.

### 5.2 API Strategy

**Primary**: Firebase HN API only.
- Real-time scores (no indexing delay).
- Story list fetches: `GET /v0/topstories.json`,
  `GET /v0/beststories.json`, `GET /v0/newstories.json`.
- Individual item fetch: `GET /v0/item/{id}.json`.
- Concurrency: 50 simultaneous requests.
- Retry: 3 attempts per story, exponential backoff.
- If an item fetch fails for an existing tracked story, keep its prior
  state unchanged and retry next cycle.
- If an item fetch fails for a newly discovered story, do not create a
  state entry; retry next cycle if it is rediscovered.
- If all three discovery endpoints fail after retries, the cycle is
  fatal and the binary MUST return exit code 1.
- No Algolia fallback in v1. A future degraded mode may be added in a
  later spec revision.

### 5.3 Processing Pipeline

```
 1. Capture `cycle_time` at process start
 2. Load state.json
 3. Cleanup: remove threshold timestamps older than `cycle_time - 7 days`
    and remove stories with no thresholds left; retain `max_scores`
 4. Fetch `topstories` + `beststories` + `newstories` in parallel
 5. Build the fetch set = deduplicated discovery IDs + retained state IDs
 6. Fetch story details (50 concurrent, 3 retries each)
 7. For each successful story response:
    a. If `type` or `title` is missing/null: ignore the response;
       keep existing state if present
    b. If `type != "story"`: remove from state if present; otherwise skip
    c. If dead/deleted: remove from state, skip
    d. Normalize fields and update state
    e. Record only threshold crossings above the durable prior maximum with
       `cycle_time`, then update `max_scores`
    f. If any persisted field or threshold map changed, set
       `last_output_change_at = cycle_time`
 8. For each failed story fetch: keep existing state unchanged;
    do not create a new entry
 9. Generate 96 feed files, `index.html`, and `_headers` in a temporary
    output directory
10. Compare the next `state.json`, feed files, and `_headers` against
    the previous persisted bytes
11. If any of those bytes differ, write the new state, replace the
    output directory, and exit code 0
12. If all of those bytes are identical, leave persisted files
    unchanged and exit code 2
```

---

## 6. State Management

### 6.1 Storage

`state.json` lives in the repository root at runtime. It is the source of
truth for tracked stories and is restored from the newest snapshot in the
`state` GitHub Release at the start of each update run. Whenever its bytes
change, the workflow uploads a new snapshot before deploying and retains the
newest two snapshots. The file is not tracked by git.

### 6.2 Schema

```json
{
  "version": 1,
  "last_output_change_at": "2025-04-14T15:00:56Z",
  "max_scores": {
    "8863": 450
  },
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

| Top-level field | Description |
|-----------------|-------------|
| `version` | Schema version |
| `last_output_change_at` | Latest `cycle_time` that changed persisted state. Use `1970-01-01T00:00:00Z` when `stories` is empty. |
| `max_scores` | Durable map of string HN item ID -> highest score ever observed. Entries do not expire. |
| `stories` | Map of string HN item ID -> story object |

| Story field | Description |
|-------------|-------------|
| `id` | HN item ID |
| `title` | Story title (HTML decoded) |
| `url` | Original article URL (empty for self-posts) |
| `hn_url` | HN comments page URL |
| `score` | Current score |
| `max_score` | Highest score ever observed |
| `comments` | Current comment count (`descendants`) |
| `by` | Author username |
| `first_seen` | When first tracked |
| `story_time` | Original HN submission timestamp (Unix) |
| `last_output_change_at` | Latest `cycle_time` that changed any persisted field for this story |
| `thresholds` | Map of threshold value -> ISO 8601 crossing time |

### 6.3 Lifecycle

1. **First seen**: If the ID has no `max_scores` history, all
   currently-crossed thresholds are recorded with `cycle_time`. `first_seen`
   and story `last_output_change_at` are both set to `cycle_time`.
2. **Score/comments/title/url/by changes**: Update the field and set
   story `last_output_change_at = cycle_time`.
3. **Score increases**: Update `max_score` and `max_scores` if needed and
   record only thresholds above the durable prior maximum with `cycle_time`.
4. **Score decreases**: Update current `score`. Existing threshold
   timestamps do not change.
5. **Dead/deleted or non-story**: Remove the active story entry. Preserve its
   `max_scores` history so it cannot re-enter an old feed.
6. **Cleanup**: Remove threshold timestamps older than 7 days. Remove the
   story if it has no thresholds left, but preserve `max_scores`. `first_seen`
   is never used for expiry.
7. **Top-level timestamp**: `last_output_change_at` equals the maximum
   story `last_output_change_at`, or the Unix epoch if state is empty.

### 6.4 Cold Start

On first run (empty state), all valid discovered stories are newly
tracked. All currently crossed thresholds are recorded with `cycle_time`.
When loading an older version-1 file without `max_scores`, derive the ledger
from every retained story's `max_score` before processing new responses.

### 6.5 Size

~3,500 active story entries max (7 days x 500/day), plus one compact,
non-expiring `max_scores` entry per observed story. The ledger is intentionally
durable because guaranteed duplicate prevention requires crossing history.

---

## 7. Infrastructure

### 7.1 Hosting: Cloudflare Workers

- **Static assets**: Free, unlimited requests.
- **Cache**: 1 minute via `_headers` file.
- **Domain**: `hn.ysm.dev` (custom domain, CF-managed DNS).
- **Exact generated header rules**: See section 18.7.

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

### 7.2 Deploy Flow

```
GitHub Actions (cron: */5 * * * *)
  |
  +-> Download pre-built binary from GitHub Release
  +-> Restore state.json from the `state` GitHub Release
  |
  +-> Loop for up to 12 minutes, 60s apart:
  |     |
  |     +-> Run binary --state ./state.json --output ./dist/
  |     +-> Exit code 0 (state or feed output changed):
  |     |     +-> Upload a new `state` release snapshot
  |     |     +-> wrangler deploy
  |     +-> Exit code 1 (fatal):
  |     |     +-> Log error, continue loop
  |     +-> Exit code 2 (no persisted change):
  |     |     +-> Skip
  |     +-> Sleep 60s (except last iteration)
  |
  +-> Done
```

When `state.json` changes, it is the canonical source of truth, so its
release upload always happens before deploy.

### 7.3 Concurrency

```yaml
concurrency:
  group: update-feeds
  cancel-in-progress: false
```

New runs queue (wait) instead of canceling the current run.

### 7.4 Secrets (GitHub Secrets)

| Secret | Purpose |
|--------|---------|
| `CLOUDFLARE_API_TOKEN` | Wrangler deploy auth |
| `CLOUDFLARE_ACCOUNT_ID` | Target CF account |

### 7.5 Binary Build

Separate workflow on `src/`/`Cargo.toml`/`Cargo.lock` changes:

1. Build `x86_64-unknown-linux-gnu` on Ubuntu.
2. Run all tests + coverage check.
3. Upload binary to GitHub Release.

Update workflow downloads the pre-built binary. No Rust in update CI.

### 7.6 Failure Recovery

If `wrangler deploy` fails after a successful cycle:
- If `state.json` changed, the `state` release already contains the
  canonical updated state.
- If `state.json` did not change, the next cycle regenerates the same
  output from the newest release snapshot.
- Deploy is retried on the next successful cycle.
- Self-healing. No manual intervention.

---

## 8. Repository Structure

```
hn-scored/
├── .github/
│   └── workflows/
│       ├── build.yml
│       └── update.yml
├── src/
│   ├── main.rs            # Entry point, CLI args
│   ├── config.rs           # Constants (thresholds, limits, URLs)
│   ├── api/
│   │   ├── mod.rs          # API module
│   │   └── firebase.rs     # Firebase HN API client
│   ├── state/
│   │   ├── mod.rs          # State module
│   │   ├── store.rs        # Load/save state.json
│   │   ├── cleanup.rs      # 7-day expiry, dead/deleted removal
│   │   └── threshold.rs    # Threshold crossing logic
│   ├── feed/
│   │   ├── mod.rs          # Feed module
│   │   ├── rss.rs          # RSS 2.0 generation
│   │   ├── atom.rs         # Atom 1.0 generation
│   │   ├── json_feed.rs    # JSON Feed 1.1 generation
│   │   └── common.rs       # Shared feed utilities
│   ├── html/
│   │   ├── mod.rs          # HTML module
│   │   └── index.rs        # index.html generation
│   └── types.rs            # Shared types (Story, State, Config)
├── tests/
│   ├── unit/
│   ├── integration/
│   └── e2e/
├── wrangler.jsonc
├── _headers
├── Cargo.toml
├── Cargo.lock
├── state.json               # Runtime file restored from release (gitignored)
├── LICENSE                 # MIT
├── README.md
└── docs/
    ├── spec.en.md
    └── spec.ko.md
```

**Not in git** (deployed directly to CF):
```
dist/
├── feeds/
│   ├── article/
│   │   ├── 0.xml, 0.atom, 0.json
│   │   └── ... (16 thresholds x 3 formats)
│   └── comments/
│       ├── 0.xml, 0.atom, 0.json
│       └── ...
├── index.html
└── _headers
```

### 8.1 File Size Rule

**Every file in `src/` must be 200 lines or fewer.** Enforced in CI.
If a module grows beyond 200 lines, split it. No exceptions.

---

## 9. Landing Page

### 9.1 Design

HN original style:
- Orange header (`#ff6600`).
- Monospace/system font.
- Simple HTML table.

### 9.2 Content

- Exact HTML markup is informative, not normative.
- The page MUST list all 16 thresholds in ascending order.
- Each threshold row MUST expose article/comment URLs for RSS, Atom,
  and JSON Feed.
- Title: **hn-scored**
- One-line: "Hacker News stories filtered by score. Pick a threshold and subscribe."
- Table: Threshold | Article (RSS / Atom / JSON) | Comments (RSS / Atom / JSON)
- Copy button per URL.
- Footer: `Last feed change: {state.last_output_change_at}` + GitHub repo link.

If a footer timestamp is shown, it MUST be derived from the state-level
`last_output_change_at`, not the current render time.

---

## 10. Logging

### 10.1 Summary Line (per cycle)

```
[2025-04-14T12:34:56Z] fetched=823 new=12 crossings=45 dead=2 changed=true duration=3.2s
```

### 10.2 Warnings

```
[WARN] fetch failed: item 12345 (attempt 3/3): connection timeout
```

---

## 11. Testing

### 11.1 Rules

- **Red/Green TDD**: Write the failing test first. Then make it pass.
- **Coverage >= 90%**: Enforced in CI. Build fails below 90%.
- **All paths tested**: Normal, edge, and error paths.

### 11.2 Unit Tests

| Module | Covers |
|--------|--------|
| `feed/rss.rs` | RSS generation, empty feeds, max items, HTML entities, special chars |
| `feed/atom.rs` | Atom generation, same edge cases |
| `feed/json_feed.rs` | JSON Feed generation, same edge cases |
| `state/store.rs` | Load, save, deterministic serialization order, invalid-state recovery |
| `state/cleanup.rs` | 7-day threshold expiry, empty-story removal |
| `state/threshold.rs` | Crossing detection, once-included policy, cold start, tie-break ordering |
| `api/firebase.rs` | List/item parsing, retry, partial failure handling |
| `html/index.rs` | HTML generation, threshold ordering, footer timestamp sourcing |

### 11.3 Integration Tests

Full pipeline with mocked HTTP:
- Stories rising through thresholds over multiple cycles.
- Late threshold crossing remains for 7 days from crossing time.
- Score drops (story remains in feed).
- Dead/deleted removal.
- Partial network failure (some stories fail; prior state is retained).
- Full discovery-endpoint outage (fatal exit, no persisted changes).
- Corrupted state.json recovery.
- Cold start (empty state).
- Max 200 items limit with deterministic tie-break.
- Unchanged cycle produces byte-identical state/feed output.
- 7-day cleanup of threshold timestamps and empty stories.

### 11.4 E2E Tests

- Single cycle against real HN API.
- Validates well-formed RSS/Atom/JSON output.
- Validates state.json round-trip.

---

## 12. CI Configuration

### 12.1 build.yml

Triggered on: `src/**`, `Cargo.toml`, `Cargo.lock` changes.

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. Coverage check (>= 90%)
5. Line count check (no file > 200 lines)
6. `cargo build --release`
7. Upload binary to GitHub Release

### 12.2 update.yml

Triggered on: `schedule: */5 * * * *` and `workflow_dispatch`.

1. Checkout repo.
2. Download pre-built binary from Release.
3. Install wrangler.
4. Run 5-iteration loop (details in section 7.2).

---

## 13. Wrangler Configuration

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

## 14. Exact Format Specifications

### 14.1 Atom 1.0 Example

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

| Atom Field | Value |
|------------|-------|
| `<feed><id>` | Self URL of the feed. |
| `<feed><updated>` | Latest `last_output_change_at` among rendered entries, or `1970-01-01T00:00:00Z` if the feed is empty. Same instant as RSS `<lastBuildDate>`. |
| `<feed><link rel="self">` | URL of this feed. |
| `<entry><id>` | Same as RSS `<guid>`: HN item URL. |
| `<entry><title>` | Same rule as RSS `<title>` (see 4.4 and 18.11). |
| `<entry><updated>` | Story `last_output_change_at`. |
| `<entry><published>` | Threshold crossing time for this feed. |
| `<entry><author><name>` | HN username. Omit `<author>` entirely if `by` is empty. |
| `<entry><summary>` | Same as RSS `<description>`. Plain text. |

### 14.2 JSON Feed 1.1 Example

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

| JSON Feed Field | Article feed | Comments feed |
|-----------------|-------------|---------------|
| `items[].id` | HN item URL | HN item URL |
| `items[].title` | Same rule as RSS `<title>` (see 4.4 and 18.11). | Same |
| `items[].url` | Original article URL | HN comments URL |
| `items[].external_url` | HN comments URL | Original article URL (swapped) |
| `items[].content_text` | Plain text. Same as RSS description. | Same |
| `items[].date_published` | ISO 8601 UTC. Threshold crossing time. | Same |
| `items[].date_modified` | ISO 8601 UTC. Story `last_output_change_at`. | Same |
| `items[].authors` | `[{"name": "hn_username"}]`. Omit the field if `by` is empty. | Same |

Note: `url` and `external_url` are **swapped** between article and
comments feeds. In article feed, `url` = article, `external_url` = HN.
In comments feed, `url` = HN, `external_url` = article.
For self-posts (no article URL), both point to HN comments URL.

### 14.3 All Timestamps Are UTC

- RSS: RFC 2822 format. Example: `Mon, 14 Apr 2025 12:34:56 +0000`
- Atom: ISO 8601 / RFC 3339. Example: `2025-04-14T12:34:56Z`
- JSON Feed: ISO 8601 / RFC 3339. Example: `2025-04-14T12:34:56Z`
- state.json: ISO 8601. Example: `2025-04-14T12:34:56Z`

No local timezones. Always UTC with `Z` suffix or `+0000`.
Every new timestamp created during a cycle MUST use the captured
`cycle_time`.

### 14.4 Text Encoding

- All feed content is **plain text**, not HTML.
- `<description>` / `<summary>` / `content_text` are plain text.
- Story titles from HN API may contain HTML entities (`&amp;`, `&#x27;`, `&lt;`).
  These must be **decoded to plain text** before storing in state and feeds.
- When writing XML (RSS/Atom), plain text is then XML-escaped
  (`&` -> `&amp;`, `<` -> `&lt;`, `>` -> `&gt;`, `"` -> `&quot;`).
- JSON Feed: plain text is JSON-escaped (standard `serde_json` behavior).

---

## 15. Edge Cases & Exact Behaviors

### 15.1 Threshold 0 Semantics

The `0` threshold feed includes **every valid story in the fetch set**,
regardless of score. A story with score 0 or even negative is included.
The only filter is `type == "story"` and not dead/deleted.

### 15.2 Missing or Null API Fields

HN API items may have missing fields. Exact handling:

| Field | If missing/null |
|-------|-----------------|
| `score` | Treat as 0. |
| `title` | Invalid response. Do not create a new story. If the story already exists, keep the previous stored entry unchanged. |
| `url` | Treat as empty string (self-post). |
| `type` | Invalid response. Do not create a new story. If the story already exists, keep the previous stored entry unchanged. |
| `dead` | Treat as false. |
| `deleted` | Treat as false. |
| `descendants` | Treat as 0 (comments count). |
| `by` | Use empty string. |
| `time` | Use 0 (Unix epoch). |

If a successful response has `type != "story"`, remove any existing
state entry for that ID. Missing/null `type` is not a delete signal;
it is an invalid response.

### 15.3 URL Domain Extraction for Description

Given a URL, extract the domain+path for the description field:

1. Parse URL. If parsing fails, use the raw URL string.
2. Remove scheme (`https://`, `http://`).
3. Remove `www.` prefix if present.
4. Remove trailing `/` if the path is just `/`.
5. Remove query string (`?...`) and fragment (`#...`).
6. Keep port if non-standard (not 80/443).

Examples:
```
https://www.github.com/foo/bar?ref=hn  -> github.com/foo/bar
https://blog.example.com:8080/post     -> blog.example.com:8080/post
https://example.com/                   -> example.com
https://example.com                    -> example.com
```

### 15.4 Self-post Description Domain

For self-posts (empty URL), the description domain field shows
`news.ycombinator.com/item?id={id}`.

Example: `42 points | 15 comments | news.ycombinator.com/item?id=12345`

### 15.5 What Constitutes "Changed" (Exit Code)

Exit code 0 (changed) is returned when **any of the following** differ
from the previous persisted bytes:

- The next `state.json` bytes.
- Any generated feed file bytes.
- The generated `_headers` bytes.
- Absence of a previously required file.

Exit code 2 (no change) means `state.json`, all feed files, and
generated `_headers` are **byte-identical** to the previous persisted
versions.

`index.html` is informative and does not affect conformance.

Exit code 1 means a fatal error occurred (see 15.10).

### 15.6 state.json Format Rules

- **Pretty-printed** with 2-space indentation (human-readable git diffs).
- **UTF-8** encoding, no BOM.
- **LF line endings** and a trailing newline at end of file.
- Top-level key order is exactly: `version`, `last_output_change_at`,
  `max_scores`, `stories`.
- `max_scores` is keyed by string ID and sorted by numeric ID ascending.
- Stories are keyed by string ID (`"8863"`, not `8863`) sorted by
  numeric ID ascending.
- Story object key order is exactly: `id`, `title`, `url`, `hn_url`,
  `score`, `max_score`, `comments`, `by`, `first_seen`, `story_time`,
  `last_output_change_at`, `thresholds`.
- `thresholds` keys are strings sorted by numeric threshold ascending.

### 15.7 Corrupted or Missing state.json

| Condition | Behavior |
|-----------|----------|
| File does not exist | Start with empty state (cold start). |
| File is empty (0 bytes) | Start with empty state (cold start). |
| File contains invalid JSON | Log error, start with empty state. |
| File has wrong `version` | Log error, start with empty state. |
| File is valid but some entries malformed | Skip malformed entries, keep valid ones. |

Never crash on bad state. Always recover gracefully.

### 15.8 Retry Backoff

Exponential backoff with jitter for individual story fetches:

| Attempt | Base delay | With jitter |
|---------|-----------|-------------|
| 1 (initial) | 0ms | 0ms |
| 2 (1st retry) | 500ms | 250-750ms |
| 3 (2nd retry) | 1000ms | 500-1500ms |

Formula: `base_delay * 2^(attempt-2)` with +/- 50% random jitter.
Max delay capped at 2 seconds.

### 15.9 Discovery Endpoint Failure

- If one or two discovery endpoints fail after retries, continue the
  cycle with the successful discovery lists plus retained state IDs.
- If all three discovery endpoints fail after retries, return exit code 1
  and leave persisted state/output untouched.
- There is no secondary discovery provider in v1.

### 15.10 Exit Codes

| Code | Meaning | Action in shell |
|------|---------|-----------------|
| 0 | `state.json`, feed files, or `_headers` changed. Persisted files written. | Commit state if needed, then deploy. |
| 1 | Fatal error (can't write files, etc.). | Log error, continue loop. |
| 2 | No persisted changes. | Skip deploy. |

---

## 16. CLI Interface

### 16.1 Binary Name

`hn-scored` (hyphenated, matching the crate/repo name).

### 16.2 Arguments

```
hn-scored --state <PATH> --output <PATH>

Options:
  --state <PATH>    Path to state.json [default: ./state.json]
  --output <PATH>   Output directory for feeds [default: ./dist]
  --base-url <URL>  Base URL for self-referencing links
                    [default: https://hn.ysm.dev]
  --help            Print help
  --version         Print version
```

No other flags. No verbose/quiet flags (log level is fixed).

`--base-url` MUST be an absolute `http` or `https` URL.
Before generating self-referential URLs, remove exactly one trailing
slash if present. The normalized value is used for RSS
`<atom:link rel="self">`, Atom feed IDs/self links, and JSON Feed
`feed_url`.

---

## 17. CI Operational Details

### 17.1 Runtime State Persistence

The update workflow downloads `state.json` from the newest snapshot in the
rolling `state` release. On exit code 0, it uploads a uniquely named snapshot
before deploying, then deletes all but the newest two snapshots. Uploading the
replacement before deleting older snapshots prevents a failed GitHub API call
from removing the only recoverable state. A missing release is created, and a
release with no snapshot cold-starts from empty state.

### 17.2 GitHub Release Tags

The `latest` release contains the pre-built binary. The build workflow
**deletes and recreates** it on each successful build. The separate `state`
release contains the rolling runtime state and is never deleted by the build
workflow.

### 17.3 Wrangler Version

Pinned in workflow:
```yaml
- run: npm install -g wrangler@4
```

Major version pinned, minor/patch float.

### 17.4 GitHub Actions Permissions

```yaml
permissions:
  contents: write    # For release upload
```

### 17.5 E2E Tests in CI

E2E tests that call real HN API are **excluded from the build workflow**.
They run only via `cargo test --ignored` manually or in a separate
nightly workflow. This prevents external API flakiness from blocking
releases.

Integration tests with mocked HTTP run in the build workflow.

---

## 18. Remaining Exact Behaviors

### 18.1 Number Formatting in Description

No thousands separator. Plain integers.
- Correct: `1234 points | 567 comments`
- Wrong: `1,234 points | 567 comments`

### 18.2 Empty Feeds

If a threshold has 0 stories (e.g., threshold 1000 with no stories
crossing 1000 in 7 days), an **empty feed file is still generated**.
It contains the channel/feed metadata but zero items/entries.
Never skip generating a file.

For empty feeds, use the Unix epoch for feed-level timestamps:
- RSS `<lastBuildDate>` = `Thu, 01 Jan 1970 00:00:00 +0000`
- Atom `<feed><updated>` = `1970-01-01T00:00:00Z`

### 18.3 Output Directory Handling

1. Each cycle generates the complete output in a temporary directory.
2. Required directory structure: `dist/feeds/article/`,
   `dist/feeds/comments/`.
3. Required files: 96 feed files, `dist/index.html`, and `dist/_headers`.
   No extra files are allowed in the final directory.
4. The binary MUST NOT leave behind a partially-written `dist/`
   directory. Replace the output directory only after full generation
   succeeds.
5. The exact swap mechanism is implementation-defined, but the final
   on-disk result MUST be atomic from the reader's perspective.
6. The `_headers` file is generated by the binary, not copied from the
   repo root.

### 18.4 Change Detection on First Run

On first run (no previous persisted `state.json`, feed output, or
generated `_headers`), the cycle is considered "changed".
Exit code 0.

Absence of any required persisted file counts as a difference during the
byte comparison step.

### 18.5 200-Line Limit Enforcement

- Counted by: `wc -l` (total newline characters).
- **Includes**: blank lines, comments, doc comments, attributes.
- **Applies to**: all `*.rs` files under `src/`.
- **Does not apply to**: `tests/`, `build.rs`, `Cargo.toml`.
- CI check command:
  ```bash
  find src -name '*.rs' | while read f; do
    lines=$(wc -l < "$f")
    if [ "$lines" -gt 200 ]; then
      echo "FAIL: $f has $lines lines (max 200)" && exit 1
    fi
  done
  ```

### 18.6 `--version` Output

Prints the version from `Cargo.toml`:
```
hn-scored 0.1.0
```

### 18.7 `_headers` Content (Generated by Binary)

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

This ensures Cloudflare Workers serves correct MIME types for all
extensions including `.atom` (which is not a standard MIME mapping).

### 18.8 RSS Channel Link for Comments Feed

The `<channel><link>` is always `https://news.ycombinator.com`
for both article and comments feeds. It represents the website
the feed is about, not the feed itself. The feed's self-link is
in `<atom:link rel="self">`.

### 18.9 workers.dev URL

The `workers_dev` setting in `wrangler.jsonc` is `true`. Both URLs work:
- `https://hn.ysm.dev/...` (custom domain)
- `https://hn-scored.{subdomain}.workers.dev/...` (default)

### 18.10 Landing Page Copy Button

JavaScript Clipboard API (`navigator.clipboard.writeText()`).
Inline `<script>` in the HTML. No external JS dependencies.
If JS is disabled, the URL is still visible as plain text and
can be manually copied. No functionality loss, just no button.

### 18.11 Title Elapsed-Time Suffix

RSS `<title>`, Atom `<entry><title>`, and JSON Feed `items[].title` all
append how long the story took to reach **this feed's threshold** after
being posted to HN.

**Formula**:

```
total_minutes = floor((thresholds[N] - story_time) in minutes), clamped to >= 0
days    = total_minutes / (24 * 60)
hours   = (total_minutes % (24 * 60)) / 60
minutes = total_minutes % 60
```

Where `N` is the threshold of the feed being rendered (`thresholds[N]` is
the same instant already used as `<pubDate>` / `<published>` /
`date_published` for that item).

**Rendering**:
- Build the suffix as `"{days}d {hours}h {minutes}m"`.
- The `d` segment is omitted entirely if `days == 0`.
- The `h` segment is omitted entirely if `hours == 0`.
- The `m` segment is **always** present, even when `0`.
- Final title: `"{title} ({suffix})"`.

Examples: `(2d 3h 32m)`, `(3h 32m)`, `(32m)`, `(1d 0m)`, `(1h 0m)`.

**Exclusions** (title is the plain, unmodified HN title in these cases):
- The threshold-0 "All Stories" feed. Threshold 0 is crossed at
  first-seen time, so this figure would be near-zero noise on every item.
- Any story with `story_time == 0` (the "missing `time` field" fallback
  from 15.2). Computing elapsed time against the Unix epoch would produce
  a meaningless multi-decade figure.

This suffix does not change once written: `thresholds[N]` is write-once,
expired thresholds cannot be recreated because `max_scores` is durable, and
`story_time` does not change after a story is first tracked under normal
operation. Unlike raw score, this value is safe to bake into a title that feed
readers may cache.

---

## 19. License

MIT License.
