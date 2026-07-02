//! Decision output.

use bridge_types::Card;
use std::time::Duration;

/// A card choice plus everything a caller could want to know about it.
///
/// The extra fields are cheap to produce inside the decision and awkward to
/// reconstruct outside it, so they ride along:
/// - `rule` / `explanation` power the teaching UI ("why did East play the 9?")
///   and rule-level telemetry,
/// - `legal_count` enables statistical analysis of how constrained each
///   decision was (a bot that's right with 1 legal card proves nothing),
/// - `duration_micros` means callers never have to time the bot externally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// The chosen card — always a member of the `legal` set that was passed
    /// in.
    pub card: Card,
    /// Stable slug for the rule that fired, e.g. `"third-hand-high"`,
    /// `"attitude-encourage"`, `"fallback-lowest"`. Slugs are an API
    /// contract: telemetry and tests key on them, so they never change once
    /// shipped (the human wording in `explanation` may).
    pub rule: &'static str,
    /// Human-readable, student-facing sentence for why this card.
    pub explanation: String,
    /// How many legal cards the bot chose among.
    pub legal_count: usize,
    /// How long the decision took, in microseconds. Measured internally
    /// around the rule pipeline; expressed as an integer so the value
    /// crosses the future WASM boundary without a Duration type.
    pub duration_micros: u64,
}

impl Decision {
    pub(crate) fn new(
        card: Card,
        rule: &'static str,
        explanation: String,
        legal_count: usize,
        elapsed: Duration,
    ) -> Self {
        Decision {
            card,
            rule,
            explanation,
            legal_count,
            duration_micros: elapsed.as_micros() as u64,
        }
    }
}
