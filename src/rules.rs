//! The rule pipeline.
//!
//! Each entry point walks an ordered list of rules and takes the first one
//! that fires. Order encodes priority; every rule is a pure function of
//! (context, config). The V1 rules from docs/requirements.md land here one
//! at a time, each with its worked examples as tests; until a situation is
//! covered by a real rule, the always-correct fallbacks below keep the bot
//! legal and deterministic (never random — random play would corrupt the
//! signaling story partners rely on).

use crate::config::SignalConfig;
use crate::context::{LeadContext, PlayContext};
use crate::{lowest, BotError};
use bridge_types::Card;

/// Outcome of a fired rule: the card, the rule slug, the student-facing
/// explanation.
pub(crate) type Fired = (Card, &'static str, String);

/// Opening-lead pipeline.
///
/// V1 target order (requirements §Opening leads): partner's bid suit →
/// top of sequence → A from AK → 4th best from length → low from honor-third
/// → top of nothing/doubleton, with vs-suit / vs-NT variations. None are
/// implemented yet; the fallbacks below apply.
pub(crate) fn opening_lead(ctx: &LeadContext, _config: &SignalConfig) -> Result<Fired, BotError> {
    forced(&ctx.legal)
        .or_else(|| fallback_lowest(&ctx.legal))
        .ok_or(BotError::NoLegalCards)
}

/// Mid-play pipeline.
///
/// V1 target order (requirements §Following suit): win cheaply when the
/// trick can be won → second hand low / cover an honor → third hand high →
/// attitude signal on partner's lead → count signal on declarer's lead →
/// suit-preference / attitude discards → fallback.
pub(crate) fn play(ctx: &PlayContext, _config: &SignalConfig) -> Result<Fired, BotError> {
    forced(&ctx.legal)
        .or_else(|| fallback_lowest(&ctx.legal))
        .ok_or(BotError::NoLegalCards)
}

/// Only one legal card: no decision to make.
fn forced(legal: &[Card]) -> Option<Fired> {
    match legal {
        [only] => Some((*only, "forced", "Only legal card.".to_string())),
        _ => None,
    }
}

/// Terminal fallback: the lowest legal card. Deterministic and never wastes
/// an honor; fires only when no real rule covered the situation.
fn fallback_lowest(legal: &[Card]) -> Option<Fired> {
    lowest(legal).map(|card| {
        (
            card,
            "fallback-lowest",
            "No specific rule applies; playing the lowest legal card.".to_string(),
        )
    })
}
