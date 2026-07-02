//! Rule-based cardplay bot for bridge teaching.
//!
//! Sits between `RandomLegalBot` (too random) and BEN (too slow to start):
//! a deterministic, sub-millisecond bot that follows the basic guidelines a
//! bridge teacher actually teaches — opening leads, second-hand-low,
//! third-hand-high, and (the differentiator) honest defensive signals.
//!
//! Every decision returns a [`Decision`] carrying not just the card but the
//! rule that chose it (a stable slug plus a human-readable explanation), the
//! number of legal cards it selected from, and the time the decision took.
//! The reason string is a first-class teaching feature: the UI can explain
//! *why* East played the 9.
//!
//! # Statelessness
//!
//! The bot holds no memory between calls. Each call receives the full play
//! history ([`PlayContext::played`]) plus the original visible hands, and
//! derives anything it needs (dummy's remaining cards, who has shown out)
//! internally. This is deliberate: the table service's bot driver re-decides
//! from a folded snapshot every iteration to stay undo-safe, and the solo
//! client passes full history on every call — a bot that tracked its own
//! state would silently desync on undo or reconnect.
//!
//! # Rule status
//!
//! The V1 rule set from docs/requirements.md §5 is implemented: opening
//! leads, second-hand play, third-hand play, attitude/count signals (all
//! four method combinations), ruffs, discards, defender continuation, and
//! minimal declarer play. Each rule's worked example from the requirements
//! doc runs as a test in tests/rules.rs. The always-correct terminals
//! (`forced`, `fallback-lowest`) keep the pipeline total.

mod config;
mod context;
mod decision;
mod derived;
mod rules;

pub use config::{AttitudeMethod, CountMethod, SignalConfig};
pub use context::{LeadContext, PlayContext, PlayedCard};
pub use decision::Decision;

use bridge_types::Card;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

/// Choose an opening lead.
///
/// `ctx.legal` is the leader's full remaining hand (any card may be led).
/// Returns the chosen card with the rule that selected it. Never fails on a
/// non-empty legal set; an empty legal set is an engine bug and returns the
/// `Err` variant rather than panicking.
pub fn choose_opening_lead(ctx: &LeadContext, config: &SignalConfig) -> Result<Decision, BotError> {
    let start = Instant::now();
    let (card, rule, explanation) = rules::opening_lead(ctx, config)?;
    Ok(Decision::new(
        card,
        rule,
        explanation,
        ctx.legal.len(),
        start.elapsed(),
    ))
}

/// Choose a card mid-play (any trick after the opening lead).
///
/// `ctx.legal` must be the engine's pre-filtered legal subset; the bot only
/// ever returns a member of it.
pub fn choose_card(ctx: &PlayContext, config: &SignalConfig) -> Result<Decision, BotError> {
    let start = Instant::now();
    let (card, rule, explanation) = rules::play(ctx, config)?;
    Ok(Decision::new(
        card,
        rule,
        explanation,
        ctx.legal.len(),
        start.elapsed(),
    ))
}

/// The only failure mode: the caller passed no legal cards. The engine on
/// both seams (solo client and table service) guarantees a non-empty legal
/// set, so seeing this error means the *caller* is broken, not the bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotError {
    /// `legal` was empty.
    NoLegalCards,
}

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotError::NoLegalCards => write!(f, "legal card set is empty (engine bug)"),
        }
    }
}

impl std::error::Error for BotError {}

/// Lowest card in a set, ordered by rank then suit (suit only breaks exact
/// rank ties, e.g. discarding from equals — deterministic either way).
pub(crate) fn lowest(cards: &[Card]) -> Option<Card> {
    cards.iter().copied().min_by_key(|c| (c.rank, c.suit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_types::{Card, Direction, Rank, Suit, Vulnerability};

    fn card(suit: Suit, rank: Rank) -> Card {
        Card::new(suit, rank)
    }

    fn lead_ctx(legal: Vec<Card>) -> LeadContext {
        LeadContext {
            seat: Direction::West,
            hand: legal.clone(),
            declarer: Direction::South,
            dealer: Direction::North,
            contract: "3NT".to_string(),
            auction: vec![],
            vulnerability: Vulnerability::None,
            legal,
        }
    }

    fn play_ctx(legal: Vec<Card>) -> PlayContext {
        PlayContext {
            seat: Direction::East,
            hand: legal.clone(),
            dummy: vec![],
            declarer: Direction::South,
            dealer: Direction::North,
            contract: "3NT".to_string(),
            auction: vec![],
            vulnerability: Vulnerability::None,
            played: vec![],
            legal,
        }
    }

    #[test]
    fn forced_card_when_single_legal() {
        let only = card(Suit::Hearts, Rank::Seven);
        let d = choose_card(&play_ctx(vec![only]), &SignalConfig::default()).unwrap();
        assert_eq!(d.card, only);
        assert_eq!(d.rule, "forced");
        assert_eq!(d.legal_count, 1);
    }

    #[test]
    fn fallback_picks_lowest_legal() {
        let legal = vec![
            card(Suit::Spades, Rank::King),
            card(Suit::Spades, Rank::Four),
            card(Suit::Spades, Rank::Nine),
        ];
        let d = choose_card(&play_ctx(legal), &SignalConfig::default()).unwrap();
        assert_eq!(d.card, card(Suit::Spades, Rank::Four));
        assert_eq!(d.rule, "fallback-lowest");
        assert_eq!(d.legal_count, 3);
    }

    #[test]
    fn empty_legal_is_an_error_not_a_panic() {
        assert_eq!(
            choose_card(&play_ctx(vec![]), &SignalConfig::default()),
            Err(BotError::NoLegalCards)
        );
    }

    #[test]
    fn decision_reports_duration_and_reason() {
        let d = choose_opening_lead(
            &lead_ctx(vec![card(Suit::Clubs, Rank::Two)]),
            &SignalConfig::default(),
        )
        .unwrap();
        assert!(!d.explanation.is_empty());
        // Duration is measured internally; zero micros is fine (it's fast),
        // the field just has to be populated and non-negative by type.
        let _ = d.duration_micros;
    }

    /// The four signaling combinations must all be constructible and produce
    /// legal, deterministic output through the full public API. (Rule-level
    /// assertions per combination live next to each signal rule as it lands —
    /// see docs/requirements.md §Signals for the worked matrix.)
    #[test]
    fn all_four_signal_combinations_produce_legal_cards() {
        let combos = [
            SignalConfig {
                attitude: AttitudeMethod::Standard,
                count: CountMethod::Standard,
            },
            SignalConfig {
                attitude: AttitudeMethod::Standard,
                count: CountMethod::UpsideDown,
            },
            SignalConfig {
                attitude: AttitudeMethod::UpsideDown,
                count: CountMethod::UpsideDown,
            },
            // Nobody plays upside-down count with standard attitude, but the
            // config space allows it and the bot must still behave.
            SignalConfig {
                attitude: AttitudeMethod::UpsideDown,
                count: CountMethod::Standard,
            },
        ];
        let legal = vec![
            card(Suit::Diamonds, Rank::Queen),
            card(Suit::Diamonds, Rank::Eight),
            card(Suit::Diamonds, Rank::Three),
        ];
        for config in &combos {
            let ctx = play_ctx(legal.clone());
            let d = choose_card(&ctx, config).unwrap();
            assert!(
                ctx.legal.contains(&d.card),
                "combo {config:?} returned illegal card"
            );
            // Determinism: same input, same output.
            let d2 = choose_card(&ctx, config).unwrap();
            assert_eq!(d.card, d2.card);
        }
    }
}
