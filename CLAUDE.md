# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

Rule-based cardplay bot library for Bridge Classroom. Pure Rust, no I/O,
stateless: `(context, SignalConfig) → Decision { card, rule, explanation,
legal_count, duration_micros }`. Consumed natively by `bridge-table-service`
and (planned) by the Bridge-Classroom frontend via a WASM wrapper crate.

Read these before changing behavior:

- [docs/requirements.md](docs/requirements.md) — the V1 rule set with worked
  examples (each example becomes a test), the signal test matrix (all four
  attitude×count combinations), and the output contract.
- [docs/architecture.md](docs/architecture.md) — pipeline design,
  statelessness rationale, WASM constraints, dependency policy.

## Hard Invariants

- **Deterministic** — no RNG, ever. Random play corrupts defensive signals.
- **Stateless** — no memory between calls; derive everything from `played`.
- **Always legal** — returned card must be a member of `ctx.legal`.
- **Rule slugs are API** — never rename a shipped slug; `explanation`
  wording may change freely.
- **wasm-clean core** — no wasm-bindgen, serde, tokio, or std::time::Instant
  (use the existing cfg'd `web-time` import) in this crate;
  `cargo check --target wasm32-unknown-unknown` must pass.

## Build & Test Commands

```bash
cargo test                                   # Run all tests
cargo clippy --all-targets -- -D warnings    # Lint (warnings are errors)
cargo fmt --check                            # Check formatting
cargo check --target wasm32-unknown-unknown  # WASM cleanliness
```

## Pre-commit Requirements

Before committing, always run and fix:
1. `cargo fmt --all`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `cargo check --target wasm32-unknown-unknown`

## Code Standards

- No `unwrap()` or `expect()` outside test code
- No `println!()` in library code
- All public functions must have doc comments (`///`)
- Prefer editing existing files over creating new ones

## Git Configuration

Use SSH for all GitHub operations:
- Remote: `git@github.com:bridge-craftwork/bridge-rulebot.git`

## Related Projects

- `../bridge-types` — sibling crate providing `Card`, `Direction`, etc.
  (path-patched; see `[patch]` in `Cargo.toml`)
- `../bridge-table-service` — primary native consumer (`src/bots.rs`)
- `../Bridge-Classroom` — frontend; pluggable-bot seam is
  `src/utils/cardplayBots.js`
