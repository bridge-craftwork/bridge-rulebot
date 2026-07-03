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

**Use `./dev-build.sh` for local development builds, not bare cargo.** This repo depends on the sibling `bridge-types` crate as a git dependency, with a gitignored `[patch]` override in `.cargo/config.toml` redirecting it to the local checkout in `../bridge-types`. Cargo never lets a `[patch]` override an existing `Cargo.lock` pin, so bare `cargo build` silently compiles the GitHub revision instead of your local edits — and if the patch does take effect, it rewrites `Cargo.lock` with a local-path entry that must never be committed (CI has no sibling checkouts). The script keeps a separate local lock (`.cargo/dev.lock`), swaps it in around the cargo call, verifies the patched crate resolved to the local checkout, and leaves the committed `Cargo.lock` untouched.

```bash
./dev-build.sh test                                   # Run all tests
./dev-build.sh clippy --all-targets -- -D warnings    # Lint (warnings are errors)
./dev-build.sh check --target wasm32-unknown-unknown  # WASM cleanliness
cargo fmt --check                                     # no dependency resolution; bare cargo is fine
```

For CI-parity builds (pre-commit checks, release verification) use `./dev-build.sh --ci test` (any cargo subcommand works after `--ci`) — it temporarily disables the local patches and builds with the committed lock's git pins. **Avoid bare cargo for anything that resolves dependencies** (build/test/check/run): with the patches present, a same-version patch is applied immediately and silently rewrites `Cargo.lock` to local-path entries, while a version mismatch makes the patches silently ignored — both wrong. The committed `Cargo.lock` must always pin `git+https://` sources for the internal crates; never commit a lock where those entries have lost their `source =` lines.

## Pre-commit Requirements

Before committing, always run and fix:
1. `cargo fmt --all`
2. `./dev-build.sh --ci clippy --all-targets -- -D warnings`
3. `./dev-build.sh --ci test`
4. `./dev-build.sh --ci check --target wasm32-unknown-unknown`

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
  (git dependency; patched to the local checkout via the gitignored
  `.cargo/config.toml` — use `./dev-build.sh` so the patch takes effect)
- `../bridge-table-service` — primary native consumer (`src/bots.rs`)
- `../Bridge-Classroom` — frontend; pluggable-bot seam is
  `src/utils/cardplayBots.js`
