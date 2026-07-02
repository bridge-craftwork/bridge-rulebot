# bridge-rulebot — Requirements

Status: **V1 defined, pipeline scaffolded, rules landing incrementally.**
This document is the source of truth for what the bot must do. Each rule's
worked examples below become its test cases, near-verbatim.

## 1. Purpose and positioning

Bridge Classroom currently has two cardplay bots: `RandomLegalBot` (instant
but incoherent) and BEN (strong but ~20s cold start, ~500ms warm, and a
network dependency). Commercial bots — BBO's GIB, IntoBridge's Lia, Shark's —
respond in well under a second, but represent years of engineering we don't
need to replicate, and all of them are weak at **defensive signaling**, which
is precisely what a teaching site needs to demonstrate.

bridge-rulebot sits in between: a deterministic, sub-millisecond,
rule-based player that follows the guidelines a bridge teacher actually
teaches. It does not aim to be strong. It aims to be **coherent and
explainable**: a student defending with this bot as partner should see honest
attitude/count/suit-preference signals, and the UI should be able to say
*why* the bot played every card.

## 2. Scope

- **Cardplay only.** Bidding stays with BBA (table service) / existing flows.
- **Defense first.** In solo practice the student is almost always declarer,
  so the bot's dominant job is defending coherently. Bot-as-declarer only
  matters for empty seats at multiplayer tables; V1 declarer play is minimal
  (win cheaply, cash winners, fallback), and the table service may keep
  routing declarer seats to BEN.
- **Stateless.** Full history in, one decision out, no memory between calls
  (see docs/architecture.md for why this is load-bearing).

## 3. Output contract

Every decision returns, beyond the card itself:

| Field | Type | Why |
|---|---|---|
| `card` | `Card` | Always a member of the passed-in `legal` set. |
| `rule` | `&'static str` slug | Stable identifier of the rule that fired (`"third-hand-high"`, `"attitude-encourage"`, …). Telemetry and tests key on slugs; they never change once shipped. |
| `explanation` | `String` | Student-facing sentence for the teaching UI ("Third hand plays high to force declarer's honor."). Wording may evolve freely. |
| `legal_count` | `usize` | How many legal cards the decision selected from — for statistical analysis (a "correct" play among 1 legal card proves nothing; among 7 it means something). |
| `duration_micros` | `u64` | Decision time measured internally, so callers never time the bot externally. Integer so it crosses the WASM boundary trivially. |

Failure mode: exactly one — `BotError::NoLegalCards` when the caller passes
an empty legal set (an engine bug by definition; both existing engines
guarantee non-empty). The bot never panics on caller input.

## 4. Signaling configuration

Attitude and count methods are independent options:

```rust
SignalConfig {
    attitude: Standard | UpsideDown,   // default Standard
    count:    Standard | UpsideDown,   // default Standard
}
```

- **Standard attitude**: high encourages, low discourages. Upside-down is the
  reverse.
- **Standard count**: high-low shows even length, low-high shows odd.
  Upside-down is the reverse.
- All **four combinations** are supported and carry test cases, including
  upside-down count with standard attitude, which essentially nobody plays —
  the config space allows it, so the bot must behave sensibly in it.
- Suit-preference logic is method-independent in V1 (high = higher-ranking
  suit).

### Signal test matrix

Every signal-producing rule is tested against all four combinations. The
canonical cases (these become the table-driven tests):

**Attitude** — partner leads the ♠K vs 3NT; you hold ♠972 and want to
discourage; holding ♠Q92 you want to encourage:

| attitude | encourage from Q92 | discourage from 972 |
|---|---|---|
| Standard | ♠9 | ♠2 |
| UpsideDown | ♠2 | ♠9 |

**Count** — declarer leads the ♦3 toward dummy's ♦AKQ; you hold ♦84 (even)
or ♦852 (odd) with no prospect of winning the trick:

| count | from 84 (even) | from 852 (odd) |
|---|---|---|
| Standard | ♦8 (start hi-lo) | ♦2 (start low) |
| UpsideDown | ♦4 (start low) | ♦8 (start high) |

(The four full combinations are the cross product; attitude cases must come
out identical regardless of the count setting and vice versa.)

## 5. V1 rule set

Rules are listed in pipeline priority order within each situation. First
match wins. Reason-code slugs are part of the API contract.

### 5.1 Opening leads

| Slug | Rule | Worked example |
|---|---|---|
| `lead-partners-suit` | Lead partner's bid suit (low from honor, top of doubleton, top of nothing) | Partner overcalled 1♥; holding ♥K73 lead ♥3 |
| `lead-top-of-sequence` | Top of a 3-card (or KQ/QJ/JT + 9) honor sequence | ♠KQJ52 → ♠K |
| `lead-ace-from-ak` | A from AK(x+) vs suit contracts | ♥AK73 vs 4♠ → ♥A |
| `lead-fourth-best` | 4th best from longest and strongest vs NT (also vs suits from an honor-headed suit without a better option) | ♦Q8642 vs 3NT → ♦4 |
| `lead-low-from-honor` | Low from three to an honor | ♣K73 → ♣3 |
| `lead-top-of-nothing` | Top of worthless doubleton/tripleton | ♠852 → ♠8; ♠73 → ♠7 |
| `lead-no-ace-underlead` | Constraint, vs suit contracts: never underlead an ace (demoted to another suit or lead the ace) | holding ♥A742 vs 4♠, don't lead ♥2 |
| `fallback-lowest` | No rule matched | lowest legal card |

Vs-NT and vs-suit differences are encoded inside the rules (e.g. A-from-AK
is suit-only; 4th-best is primarily NT).

### 5.2 Second hand (playing 2nd to a trick)

| Slug | Rule | Worked example |
|---|---|---|
| `second-hand-cover-honor` | Cover an honor with an honor when it can promote something | ♠J led, you hold ♠Q63 → ♠Q |
| `second-hand-split-honors` | Split touching honors when a low card is led and you hold a sequence | ♥KQ7, low led → ♥Q |
| `second-hand-low` | Otherwise, second hand plays low | ♦K84, low led → ♦4 |

### 5.3 Third hand (partner led, you play 3rd)

| Slug | Rule | Worked example |
|---|---|---|
| `third-hand-high` | Third hand plays high when partner's card isn't winning | partner leads ♠5, dummy plays low, you hold ♠K82 → ♠K |
| `third-hand-only-as-high-as-needed` | Finesse against dummy's visible cards — play the cheapest card that does the job | dummy holds ♥Q94 and plays ♥4; you hold ♥J105 → ♥10 |
| `third-hand-unblock` | Unblock a doubleton honor under partner's honor lead | partner leads ♦K (from KQ109…), you hold ♦J3 → ♦J |
| `attitude-encourage` / `attitude-discourage` | When not contesting the trick (partner's card is winning, or you can't beat what's out), signal attitude per `SignalConfig` | see §4 matrix |

### 5.4 Following to declarer's leads

| Slug | Rule | Worked example |
|---|---|---|
| `win-cheaply` | Win the trick with the cheapest sufficient card when it's right to win (e.g. 4th hand, partner not winning) | declarer's ♠Q is winning, you're last with ♠K4 and ♠A → ♠K |
| `count-signal` | When not contesting, give count per `SignalConfig` | see §4 matrix |

### 5.5 Defender continuation

| Slug | Rule | Worked example |
|---|---|---|
| `return-partners-suit` | Having won, return partner's originally led suit (top of remaining doubleton, low from 3+) | partner led ♥, you won trick 1 and hold ♥86 → ♥8 back |
| `cash-established-winner` | Cash a plainly established winner rather than breaking a new suit | — |

### 5.6 Discards (can't follow suit)

| Slug | Rule | Worked example |
|---|---|---|
| `discard-keep-winners` | Constraint: never discard a card that is currently a winner | — |
| `discard-keep-parity` | Constraint: keep length parity with dummy's visible long suit | dummy has 4 clubs headed by honors; don't pitch your 4th club |
| `discard-attitude` | Signal attitude per `SignalConfig` in the pitched suit | holding ♥Q1085 and wanting hearts, pitch ♥8 (standard) / low heart (UD) |

### 5.7 Declarer (minimal in V1)

`win-cheaply`, `cash-established-winner`, `fallback-lowest`. Anything
smarter (drawing trumps, finesse planning) is V2+; multiplayer tables may
route declarer seats to BEN instead.

## 6. Non-functional requirements

- **Deterministic**: same input + config → same output, always. No RNG
  anywhere (random play would corrupt the signaling story). If tie-breaks
  ever want variety, a seed becomes an explicit input — never ambient.
- **Fast**: target sub-millisecond, hard requirement <50ms. Synchronous,
  no I/O, no allocation-heavy work.
- **Always legal**: the returned card is a member of the caller's `legal`
  set, guaranteed by construction (rules select from `legal` only).
- **Total**: every situation produces a card; `fallback-lowest` is the
  terminal rule.
- **Portable**: compiles for native targets and `wasm32-unknown-unknown`
  (timing via `web-time` on wasm). No tokio, no wasm-bindgen in the core
  crate.

## 7. Explicitly out of scope for V1

- Declarer planning (trump management, finesses, squeezes).
- Inference from the auction beyond "partner bid a suit".
- Counting declarer's hand / discovery plays.
- Falsecarding and deception.
- Suit-preference signals beyond discards (e.g. ruff situations) — V1.5
  candidate.

These are the expensive 20% that GIB spent years on. A defender that is
*coherent* beats one that is *strong* for teaching purposes.
