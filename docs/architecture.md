# bridge-rulebot — Architecture

## One crate, two runtimes

The core problem this crate solves structurally: the same bot logic must run
**natively inside the Rust table service** (server-side bots at multiplayer
tables) and **natively inside the browser** (solo practice, no network).
Writing it twice — once in Rust, once in JS, like today's `RandomLegal` — is
a maintenance trap for logic that will be tweaked constantly.

So the core is a pure Rust library with no I/O, compiled two ways:

```
                    ┌──────────────────────────────┐
                    │  bridge-rulebot (this crate) │
                    │  pure fn(ctx, config) →      │
                    │      Decision                │
                    └──────────┬───────────────────┘
              native crate dep │        wasm32 + thin JS glue
           ┌───────────────────┴──────────────┐
           ▼                                  ▼
  bridge-table-service              Bridge-Classroom frontend
  src/bots.rs — replaces the        src/utils/cardplayBots.js —
  RandomLegal fallback; serves      RuleBot adapter conforming to the
  instantly while BEN cold-starts   existing pluggable-bot interface
```

- **Table service**: plain `[dependencies]` entry + sibling `[patch]`, zero
  HTTP hop, sub-millisecond. It becomes the always-available fallback (BEN
  suggestion validated → else rulebot → never random), and covers BEN's
  ~20s cold-start window with coherent play.
- **Frontend**: packaged for the browser via a *separate* thin wrapper crate
  (`bridge-rulebot-wasm`, future) that adds `wasm-bindgen` + JSON conversion,
  built with `wasm-pack` into an npm package. The core crate stays free of
  wasm-bindgen so native consumers pay nothing. The only wasm accommodation
  in the core is `web-time` for `Instant` (std's panics on
  `wasm32-unknown-unknown`); `cargo check --target wasm32-unknown-unknown`
  is part of CI.

## Statelessness (a hard requirement, not a style choice)

The bot holds **no state between calls**. Each call passes the complete play
history (`played: Vec<PlayedCard>`, chronological, all seats) plus the
original visible hands; the bot derives everything else internally —
dummy's remaining cards, trick boundaries, who has shown out of what. At 52
cards maximum this derivation is trivially cheap.

Why this is load-bearing on both seams:

- The table service's bot driver is **undo-safe by re-deciding**: every
  iteration folds table state under the lock, releases it, then applies the
  suggestion only if the table's `seq` is unchanged. A bot with internal
  play-tracking would silently desync on undo, human reconnect, or a
  mid-think action.
- The solo client's `cardplayBots.js` contract already passes full history
  on every call, so the JS adapter is a translation, not a re-modeling.

## Decision pipeline

`src/rules.rs` walks an ordered rule list per entry point
(`choose_opening_lead`, `choose_card`); the first rule that fires wins.
Priority is the list order — no scoring, no weights. Every rule is a pure
function `(ctx, config) → Option<(Card, slug, explanation)>`.

Two rules are terminal and always-correct, making the pipeline total:

1. `forced` — one legal card, no decision.
2. `fallback-lowest` — lowest legal card by (rank, suit). Deterministic and
   honor-preserving. **Never random**: random play would corrupt the
   defensive signals that are this bot's whole reason to exist.

Adding a V1 rule = one function + its position in the pipeline + the worked
examples from `requirements.md` §5 as table-driven tests. The rule slug is
an API contract (telemetry/tests key on it); the explanation wording is not.

## Determinism

Same `(ctx, config)` → same `Decision.card`, always. No RNG, no clock
influence on the choice (the clock only *measures* `duration_micros`). This
makes table-service replays and CI tests exact. If tie-break variety is ever
wanted (e.g. equivalent spot cards), a seed becomes an explicit context
field — never ambient randomness.

## Module layout

```
src/
├── lib.rs        # entry points, BotError, crate docs
├── config.rs     # SignalConfig { attitude, count }, 4 supported combos
├── context.rs    # LeadContext, PlayContext, PlayedCard (owned snapshots)
├── decision.rs   # Decision { card, rule, explanation, legal_count, duration_micros }
└── rules.rs      # ordered pipeline; one fn per rule as they land
```

A `derived.rs` module is expected alongside `rules.rs` once real rules land:
reconstruction of trick boundaries, remaining hands per visible seat, and
shown-out tracking from `played` — computed once per call, shared by all
rules.

## Dependency policy

- `bridge-types` (sibling crate) for `Card`/`Suit`/`Rank`/`Direction`/
  `Vulnerability`/`Call` — the shared vocabulary across all bridge-craftwork
  Rust services.
- `web-time` on wasm32 only.
- Nothing else. No serde in the core (the wasm wrapper owns JSON), no async,
  no logging framework — callers log; the `Decision` carries everything
  worth logging.
