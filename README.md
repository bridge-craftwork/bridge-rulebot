# bridge-rulebot

Deterministic, rule-based cardplay bot for [Bridge Classroom](https://bridge-classroom.com),
with teachable reason codes.

Sits between `RandomLegalBot` (instant, incoherent) and BEN (strong, slow to
start): sub-millisecond decisions that follow the guidelines a bridge teacher
actually teaches — opening leads, second-hand-low, third-hand-high, and
honest defensive signals (attitude / count / suit preference, standard or
upside-down). Every decision explains itself:

```rust
use bridge_rulebot::{choose_card, PlayContext, SignalConfig};

let decision = choose_card(&ctx, &SignalConfig::default())?;
decision.card;            // always from ctx.legal
decision.rule;            // "third-hand-high" — stable slug, telemetry/tests key on it
decision.explanation;     // "Third hand plays high…" — student-facing sentence
decision.legal_count;     // how constrained the choice was (for statistics)
decision.duration_micros; // timed internally, no external stopwatch needed
```

The bot is **stateless**: full play history in, one decision out. See
[docs/architecture.md](docs/architecture.md) for why, and
[docs/requirements.md](docs/requirements.md) for the V1 rule set and the
signal test matrix.

## Status

Pipeline, output contract, and signaling configuration are in place with
always-correct fallback rules (`forced`, `fallback-lowest`). The V1 rules
from the requirements doc land incrementally, each with its worked examples
as tests.

## Build

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo check --target wasm32-unknown-unknown   # core must stay wasm-clean
```

## Consuming this crate

Same sibling-crate pattern as the rest of the bridge-craftwork Rust repos
(`bridge-types`, `bridge-encodings`): depend on the GitHub URL, patch to a
sister directory for local development.

```toml
[dependencies]
bridge-rulebot = { git = "https://github.com/bridge-craftwork/bridge-rulebot" }

[patch."https://github.com/bridge-craftwork/bridge-rulebot"]
bridge-rulebot = { path = "../bridge-rulebot" }
```

Docker/CI consumers follow the buildx multi-context pattern documented in
`bridge-table-service` ("Sibling crate path-deps"): the container layout
mirrors the developer-Mac layout, so the one `[patch]` works everywhere.

Consumers:

- **bridge-table-service** — native dependency; rulebot replaces the
  `RandomLegal` cardplay fallback and covers BEN's cold-start window.
- **Bridge-Classroom frontend** (planned) — via a thin `bridge-rulebot-wasm`
  wrapper crate (wasm-bindgen + wasm-pack), adapted to the pluggable-bot
  interface in `src/utils/cardplayBots.js`.

## License

Public domain (Unlicense), like `bridge-types`.
