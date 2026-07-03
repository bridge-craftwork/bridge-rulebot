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

**V1 rules implemented**: opening leads (§5.1), second hand (§5.2), third
hand (§5.3), attitude + count signals in all four method combinations (§4),
win-cheaply (§5.4), defender continuation (§5.5), ruff/overruff (§5.6),
constrained attitude discards (§5.7), and minimal declarer play (§5.8).
Every worked example in the requirements doc runs as a test
([tests/rules.rs](tests/rules.rs)). Not yet integrated into consumers —
that's the next step (bridge-table-service `bots.rs`, then the WASM
wrapper).

## Build

For local development builds use `./dev-build.sh` (see CLAUDE.md — it makes
the gitignored local-checkout patches in `.cargo/config.toml` actually take
effect and keeps the committed `Cargo.lock` pinned to git sources):

```sh
./dev-build.sh test
./dev-build.sh clippy --all-targets -- -D warnings
./dev-build.sh check --target wasm32-unknown-unknown   # core must stay wasm-clean
cargo fmt --check
```

Bare `cargo test` etc. also work and build against the pinned GitHub
revisions of the sibling crates — that's what CI does.

## Consuming this crate

Same pattern as the rest of the bridge-craftwork Rust repos (`bridge-types`,
`bridge-encodings`): depend on the GitHub URL; for local development against
a sister-directory checkout, add a `[patch]` in your repo's **gitignored**
`.cargo/config.toml` (never in the committed `Cargo.toml`, and never commit a
`Cargo.lock` whose entries lost their `source = "git+…"` lines).

```toml
[dependencies]
bridge-rulebot = { git = "https://github.com/bridge-craftwork/bridge-rulebot" }

# .cargo/config.toml (gitignored), for local dev only:
[patch."https://github.com/bridge-craftwork/bridge-rulebot"]
bridge-rulebot = { path = "../bridge-rulebot" }
```

Consumers:

- **bridge-table-service** — native dependency; rulebot replaces the
  `RandomLegal` cardplay fallback and covers BEN's cold-start window.
- **Bridge-Classroom frontend** (planned) — via a thin `bridge-rulebot-wasm`
  wrapper crate (wasm-bindgen + wasm-pack), adapted to the pluggable-bot
  interface in `src/utils/cardplayBots.js`.

## License

Public domain (Unlicense), like `bridge-types`.
