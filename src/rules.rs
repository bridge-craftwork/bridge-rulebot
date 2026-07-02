//! The rule pipeline.
//!
//! Each entry point walks an ordered list of rules and takes the first one
//! that fires. Order encodes priority; every rule is a pure function of
//! (context, derived state, config). Slugs and worked examples come from
//! docs/requirements.md §5 — the tests mirror that section near-verbatim.
//! The always-correct fallbacks keep the pipeline total (never random —
//! random play would corrupt the signaling story partners rely on).

use crate::config::{AttitudeMethod, CountMethod, SignalConfig};
use crate::context::{LeadContext, PlayContext};
use crate::derived::{is_honor, is_spot, trump_suit, Derived};
use crate::{lowest, BotError};
use bridge_types::{Call, Card, Direction, Rank, Suit};

/// Outcome of a fired rule: the card, the rule slug, the student-facing
/// explanation.
pub(crate) type Fired = (Card, &'static str, String);

// ═══════════════════════════════════════════════════════════════════════
// Opening leads (requirements §5.1)
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn opening_lead(ctx: &LeadContext, _config: &SignalConfig) -> Result<Fired, BotError> {
    if ctx.legal.is_empty() {
        return Err(BotError::NoLegalCards);
    }
    if let Some(f) = forced(&ctx.legal) {
        return Ok(f);
    }

    let trump = trump_suit(&ctx.contract);
    let vs_nt = trump.is_none();

    let f = lead_partners_suit(ctx, trump)
        .or_else(|| lead_top_of_sequence(ctx, trump))
        .or_else(|| lead_ace_from_ak(ctx, trump))
        .or_else(|| lead_fourth_best(ctx, trump, vs_nt))
        .or_else(|| lead_low_from_honor(ctx, trump))
        .or_else(|| lead_top_of_nothing(ctx))
        .or_else(|| fallback_lowest(&ctx.legal));
    f.ok_or(BotError::NoLegalCards)
}

/// Cards of one suit, sorted high→low.
fn suit_cards(cards: &[Card], suit: Suit) -> Vec<Card> {
    let mut v: Vec<Card> = cards.iter().copied().filter(|c| c.suit == suit).collect();
    v.sort_by_key(|c| std::cmp::Reverse(c.rank));
    v
}

fn all_suits() -> [Suit; 4] {
    [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs]
}

/// The conventional card when leading a known-good suit: top of a sequence,
/// A from AK (vs suits), low from an honor, top of nothing/doubleton.
fn conventional_card_in_suit(
    holding: &[Card],
    trump: Option<Suit>,
) -> Option<(Card, &'static str)> {
    match holding {
        [] => None,
        [only] => Some((*only, "singleton")),
        _ => {
            if sequence_top(holding).is_some() {
                Some((holding[0], "top of the sequence"))
            } else if trump.is_some()
                && holding[0].rank == Rank::Ace
                && holding[1].rank == Rank::King
            {
                Some((holding[0], "ace from ace-king"))
            } else if holding.len() == 2 {
                Some((holding[0], "top of the doubleton"))
            } else if is_honor(holding[0]) {
                Some((*holding.last().unwrap(), "low from an honor"))
            } else {
                Some((holding[0], "top of nothing"))
            }
        }
    }
}

/// §5.1 `lead-partners-suit` — partner bid a suit; lead it.
fn lead_partners_suit(ctx: &LeadContext, trump: Option<Suit>) -> Option<Fired> {
    let partner = ctx.seat.partner();
    let suit = first_bid_suit_by(&ctx.auction, ctx.dealer, partner)?;
    if Some(suit) == trump {
        return None;
    }
    let holding = suit_cards(&ctx.legal, suit);
    // Vs a suit contract, don't underlead the ace even in partner's suit —
    // lead the ace itself instead.
    let (card, how) = if trump.is_some()
        && holding.first().is_some_and(|c| c.rank == Rank::Ace)
        && holding.len() > 1
        && holding[1].rank != Rank::King
    {
        (holding[0], "the ace, not under it")
    } else {
        conventional_card_in_suit(&holding, trump)?
    };
    Some((
        card,
        "lead-partners-suit",
        format!("Leading partner's bid suit ({}).", how),
    ))
}

/// First suit `who` named in the auction, if any.
fn first_bid_suit_by(auction: &[Call], dealer: Direction, who: Direction) -> Option<Suit> {
    let order = dealer.clockwise_from();
    auction.iter().enumerate().find_map(|(i, call)| {
        if order[i % 4] != who {
            return None;
        }
        match call {
            Call::Bid { strain, .. } => strain_suit(*strain),
            _ => None,
        }
    })
}

fn strain_suit(strain: bridge_types::Strain) -> Option<Suit> {
    use bridge_types::Strain::*;
    match strain {
        Clubs => Some(Suit::Clubs),
        Diamonds => Some(Suit::Diamonds),
        Hearts => Some(Suit::Hearts),
        Spades => Some(Suit::Spades),
        NoTrump => None,
    }
}

/// Top card of a 3+ card holding headed by a 3-card sequence with the top
/// card an honor (KQJ, QJ10, J109, 1098). AKQ counts; A from AK is its own
/// rule.
fn sequence_top(holding: &[Card]) -> Option<Card> {
    if holding.len() < 3 || !is_honor(holding[0]) {
        return None;
    }
    let (a, b, c) = (
        holding[0].rank as u8,
        holding[1].rank as u8,
        holding[2].rank as u8,
    );
    (a == b + 1 && b == c + 1).then_some(holding[0])
}

/// §5.1 `lead-top-of-sequence`.
fn lead_top_of_sequence(ctx: &LeadContext, trump: Option<Suit>) -> Option<Fired> {
    for suit in all_suits() {
        if Some(suit) == trump {
            continue;
        }
        let holding = suit_cards(&ctx.legal, suit);
        if let Some(top) = sequence_top(&holding) {
            return Some((
                top,
                "lead-top-of-sequence",
                "Leading the top of a solid sequence — it forces out higher honors safely."
                    .to_string(),
            ));
        }
    }
    None
}

/// §5.1 `lead-ace-from-ak` — vs suit contracts only.
fn lead_ace_from_ak(ctx: &LeadContext, trump: Option<Suit>) -> Option<Fired> {
    trump?;
    for suit in all_suits() {
        if Some(suit) == trump {
            continue;
        }
        let holding = suit_cards(&ctx.legal, suit);
        if holding.len() >= 2 && holding[0].rank == Rank::Ace && holding[1].rank == Rank::King {
            return Some((
                holding[0],
                "lead-ace-from-ak",
                "Leading the ace from ace-king — a safe look at dummy.".to_string(),
            ));
        }
    }
    None
}

/// §5.1 `lead-fourth-best` — longest-and-strongest vs NT (honor-headed,
/// 4+ cards). Vs suits it also applies but never from an ace-headed suit.
fn lead_fourth_best(ctx: &LeadContext, trump: Option<Suit>, vs_nt: bool) -> Option<Fired> {
    let mut candidates: Vec<Vec<Card>> = all_suits()
        .into_iter()
        .filter(|s| Some(*s) != trump)
        .map(|s| suit_cards(&ctx.legal, s))
        .filter(|h| h.len() >= 4 && is_honor(h[0]))
        .filter(|h| vs_nt || h[0].rank != Rank::Ace)
        .collect();
    // Longest first; tie-break on strength (top card), then suit for
    // determinism.
    candidates.sort_by_key(|h| std::cmp::Reverse((h.len(), h[0].rank as u8, h[0].suit as u8)));
    let best = candidates.first()?;
    Some((
        best[3],
        "lead-fourth-best",
        "Leading fourth-best from the longest and strongest suit.".to_string(),
    ))
}

/// §5.1 `lead-low-from-honor` — three cards to an honor (not the ace vs a
/// suit contract — never underlead an ace there).
fn lead_low_from_honor(ctx: &LeadContext, trump: Option<Suit>) -> Option<Fired> {
    for suit in all_suits() {
        if Some(suit) == trump {
            continue;
        }
        let holding = suit_cards(&ctx.legal, suit);
        if holding.len() == 3
            && is_honor(holding[0])
            && !(trump.is_some() && holding[0].rank == Rank::Ace)
        {
            return Some((
                holding[2],
                "lead-low-from-honor",
                "Leading low from three to an honor.".to_string(),
            ));
        }
    }
    None
}

/// §5.1 `lead-top-of-nothing` — worthless doubleton/tripleton.
fn lead_top_of_nothing(ctx: &LeadContext) -> Option<Fired> {
    for suit in all_suits() {
        let holding = suit_cards(&ctx.legal, suit);
        if (2..=3).contains(&holding.len()) && !is_honor(holding[0]) {
            return Some((
                holding[0],
                "lead-top-of-nothing",
                "Leading top of nothing — no honor to protect.".to_string(),
            ));
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════
// Mid-play (requirements §5.2–5.8)
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn play(ctx: &PlayContext, config: &SignalConfig) -> Result<Fired, BotError> {
    if ctx.legal.is_empty() {
        return Err(BotError::NoLegalCards);
    }
    if let Some(f) = forced(&ctx.legal) {
        return Ok(f);
    }

    let d = Derived::compute(ctx);

    let fired = if !d.is_defender {
        declarer_play(ctx, &d)
    } else if d.position() == 0 {
        defender_lead(ctx, &d)
    } else if following_suit(ctx, &d) {
        defender_follow(ctx, &d, config)
    } else {
        defender_void(ctx, &d, config)
    };

    fired
        .or_else(|| fallback_lowest(&ctx.legal))
        .ok_or(BotError::NoLegalCards)
}

/// True when the legal set follows the led suit (the engine already
/// enforces following; if legal cards match the led suit we're following).
fn following_suit(ctx: &PlayContext, d: &Derived) -> bool {
    let Some(led) = d.current.as_ref().and_then(|t| t.led_suit()) else {
        return false;
    };
    ctx.legal.iter().all(|c| c.suit == led)
}

// ── Defender: following to the led suit ────────────────────────────────

fn defender_follow(ctx: &PlayContext, d: &Derived, config: &SignalConfig) -> Option<Fired> {
    match d.position() {
        1 => second_hand(ctx, d, config),
        2 => third_hand(ctx, d, config),
        3 => fourth_hand(ctx, d, config),
        _ => None,
    }
}

/// §5.2 second hand: cover an honor, split honors, else low (count with
/// spot-only holdings).
fn second_hand(ctx: &PlayContext, d: &Derived, config: &SignalConfig) -> Option<Fired> {
    let led = d.current.as_ref()?.plays.first()?.card;
    let my_suit = suit_cards(&ctx.legal, led.suit);

    // `second-hand-cover-honor`: an honor (J/Q/K) was led and we can beat it.
    if (Rank::Jack..=Rank::King).contains(&led.rank) {
        if let Some(cover) = my_suit
            .iter()
            .copied()
            .filter(|c| is_honor(*c) && c.rank > led.rank)
            .min_by_key(|c| c.rank)
        {
            return Some((
                cover,
                "second-hand-cover-honor",
                "Covering an honor with an honor to promote our lower cards.".to_string(),
            ));
        }
    }

    // `second-hand-split-honors`: low card led, we hold touching honors.
    if !is_honor(led) {
        if let Some(split) = touching_honor_bottom(&my_suit) {
            return Some((
                split,
                "second-hand-split-honors",
                "Splitting touching honors so declarer can't win cheaply.".to_string(),
            ));
        }
    }

    // Nothing but spots: the card is free, so give count on declarer's
    // lead (second hand for a defender is always following an opponent).
    if my_suit.iter().all(|c| is_spot(*c)) {
        return count_signal(&my_suit, config);
    }

    let low = lowest(&my_suit)?;
    Some((
        low,
        "second-hand-low",
        "Second hand plays low — partner still gets a turn.".to_string(),
    ))
}

/// The lower of a touching honor pair (KQ → Q, QJ → J), if the holding is
/// headed by one. Aces don't split (AK plays differently) and we require
/// both cards to be honors.
fn touching_honor_bottom(holding: &[Card]) -> Option<Card> {
    if holding.len() < 2 {
        return None;
    }
    let (top, second) = (holding[0], holding[1]);
    (top.rank != Rank::Ace
        && is_honor(top)
        && is_honor(second)
        && top.rank as u8 == second.rank as u8 + 1)
        .then_some(second)
}

/// §5.3 third hand: partner led this trick.
fn third_hand(ctx: &PlayContext, d: &Derived, config: &SignalConfig) -> Option<Fired> {
    let trick = d.current.as_ref()?;
    let led = trick.led_suit()?;
    let partners_card = trick.plays.first()?.card;
    let my_suit = suit_cards(&ctx.legal, led);

    // Partner's *honor* is holding the trick (top of a sequence, ace, …):
    // keep it — unblock a doubleton honor, otherwise signal attitude.
    // A winning *spot* card is different: third hand still plays high,
    // because fourth hand would beat partner's spot cheaply.
    if d.partner_is_winning() && is_honor(partners_card) {
        if my_suit.len() == 2 && is_honor(my_suit[0]) {
            return Some((
                my_suit[0],
                "third-hand-unblock",
                "Unblocking the honor so partner's suit can run.".to_string(),
            ));
        }
        return attitude_signal(&my_suit, config);
    }

    // Contest the trick: "third hand high", but only as high as needed —
    // the bottom of our top equal-group (equals judged against played cards
    // and what dummy shows), provided that still beats the current winner.
    let candidate = d
        .bottom_of_top_equals(&my_suit, &ctx.hand)
        .filter(|c| d.cheapest_winner(&[*c]).is_some());
    if let Some(card) = candidate {
        let (slug, expl) = if card == my_suit[0] {
            (
                "third-hand-high",
                "Third hand plays high — winning the trick or forcing a top honor.",
            )
        } else {
            (
                "third-hand-only-as-high-as-needed",
                "Playing just high enough — the cards in between are all visible.",
            )
        };
        return Some((card, slug, expl.to_string()));
    }

    // Can't beat what's out there: signal attitude.
    attitude_signal(&my_suit, config)
}

/// §5.4 fourth hand: win as cheaply as possible; if partner already has
/// the trick, signal count (declarer led) or just play low.
fn fourth_hand(ctx: &PlayContext, d: &Derived, config: &SignalConfig) -> Option<Fired> {
    let led = d.current.as_ref()?.led_suit()?;
    let my_suit = suit_cards(&ctx.legal, led);

    if !d.partner_is_winning() {
        if let Some(win) = d.cheapest_winner(&my_suit) {
            return Some((
                win,
                "win-cheaply",
                "Winning the trick as cheaply as possible.".to_string(),
            ));
        }
    }

    // Partner has it, or we can't beat it: the card is free. Fourth hand
    // for a defender always follows an opponent's lead, so give count.
    count_signal(&my_suit, config)
}

// ── Defender: signals ──────────────────────────────────────────────────

/// §5.3/§4 attitude: encourage holding an honor in the suit, else
/// discourage. Standard: high spot encourages; upside-down reversed.
fn attitude_signal(my_suit: &[Card], config: &SignalConfig) -> Option<Fired> {
    let like = my_suit.iter().any(|c| is_honor(*c));
    let card = pick_signal_card(my_suit, like, config.attitude)?;
    let (slug, expl) = if like {
        (
            "attitude-encourage",
            "Encouraging — I like this suit, partner: continue it.",
        )
    } else {
        (
            "attitude-discourage",
            "Discouraging — nothing here, partner: try something else.",
        )
    };
    Some((card, slug, expl.to_string()))
}

/// Attitude card selection: the highest spot we can afford when the method
/// says "signal high", the lowest card when it says "signal low".
fn pick_signal_card(holding: &[Card], like: bool, method: AttitudeMethod) -> Option<Card> {
    let high_spot = holding
        .iter()
        .copied()
        .filter(|c| is_spot(*c))
        .max_by_key(|c| c.rank);
    let low = lowest(holding);
    let wants_high = match method {
        AttitudeMethod::Standard => like,
        AttitudeMethod::UpsideDown => !like,
    };
    if wants_high {
        high_spot.or(low)
    } else {
        low
    }
}

/// §5.4/§4 count on declarer's lead. Standard: high-low = even. The count
/// is our *remaining* holding in the suit as the suit is first played.
fn count_signal(my_suit: &[Card], config: &SignalConfig) -> Option<Fired> {
    let even = my_suit.len().is_multiple_of(2);
    let wants_high = match config.count {
        CountMethod::Standard => even,
        CountMethod::UpsideDown => !even,
    };
    let spots: Vec<Card> = my_suit.iter().copied().filter(|c| is_spot(*c)).collect();
    let card = if wants_high {
        spots
            .iter()
            .copied()
            .max_by_key(|c| c.rank)
            .or_else(|| lowest(my_suit))?
    } else {
        lowest(my_suit)?
    };
    let parity = if even { "an even" } else { "an odd" };
    Some((
        card,
        "count-signal",
        format!("Giving count — showing {parity} number of cards."),
    ))
}

// ── Defender: void in the led suit (§5.6) then discards (§5.7) ─────────

fn defender_void(ctx: &PlayContext, d: &Derived, config: &SignalConfig) -> Option<Fired> {
    ruff_rules(ctx, d).or_else(|| discard(ctx, d, config))
}

fn ruff_rules(ctx: &PlayContext, d: &Derived) -> Option<Fired> {
    let trump = d.trump?;
    let my_trumps = suit_cards(&ctx.legal, trump);
    if my_trumps.is_empty() {
        return None;
    }

    // `no-ruff-partners-winner`: partner already has the trick — don't
    // waste a trump on it. Fall through to the discard rules.
    if d.partner_is_winning() {
        return None;
    }

    let winning = d.winning()?;
    if winning.card.suit == trump {
        // An opponent ruffed: `overruff-cheaply` if we can.
        let over = my_trumps
            .iter()
            .copied()
            .filter(|c| c.rank > winning.card.rank)
            .min_by_key(|c| c.rank)?;
        Some((
            over,
            "overruff-cheaply",
            "Overruffing with the cheapest trump that wins.".to_string(),
        ))
    } else {
        // `ruff-to-win`: opponents hold the trick with a plain card.
        let low = lowest(&my_trumps)?;
        Some((
            low,
            "ruff-to-win",
            "Ruffing with the cheapest trump to win the trick.".to_string(),
        ))
    }
}

/// §5.7 discards: never a winner, keep parity with dummy's long suit,
/// then signal attitude with what's left.
fn discard(ctx: &PlayContext, d: &Derived, config: &SignalConfig) -> Option<Fired> {
    let trump = d.trump;

    // Candidate = legal discards that are safe by the two constraints.
    let dummy_long: Option<Suit> = longest_suit(&d.dummy_remaining);
    let safe: Vec<Card> = ctx
        .legal
        .iter()
        .copied()
        .filter(|c| Some(c.suit) != trump)
        .filter(|c| !d.is_master(*c, &ctx.hand, true)) // `discard-keep-winners`
        .filter(|c| {
            // `discard-keep-parity`: don't shorten below dummy's length in
            // its long suit.
            match dummy_long {
                Some(s) if c.suit == s => {
                    let dummy_len = d.dummy_remaining.iter().filter(|x| x.suit == s).count();
                    let my_len = ctx.hand.iter().filter(|x| x.suit == s).count();
                    my_len > dummy_len
                }
                _ => true,
            }
        })
        .collect();
    let pool = if safe.is_empty() { &ctx.legal } else { &safe };

    // `discard-attitude`: prefer an *encouraging* pitch from a suit we like
    // (honor-headed with a spare spot to signal with — requirements §5.7:
    // holding ♥Q1085 and wanting hearts, pitch the ♥8). Otherwise a
    // discouraging card from our weakest suit.
    let by_suit: Vec<(Suit, Vec<Card>)> = all_suits()
        .into_iter()
        .map(|s| (s, suit_cards(pool, s)))
        .filter(|(_, cs)| !cs.is_empty())
        .collect();

    let liked = by_suit.iter().find(|(s, cs)| {
        let full_len = ctx.hand.iter().filter(|c| c.suit == *s).count();
        full_len >= 3
            && ctx.hand.iter().any(|c| c.suit == *s && is_honor(*c))
            && cs.iter().any(|c| is_spot(*c))
    });
    if let Some((_, cs)) = liked {
        let card = pick_signal_card(cs, true, config.attitude)?;
        return Some((
            card,
            "discard-attitude",
            "Discarding an encouraging card — partner, lead this suit.".to_string(),
        ));
    }

    let (_, weakest) = by_suit.iter().min_by_key(|(s, cs)| {
        let honors = cs.iter().filter(|c| is_honor(**c)).count();
        (honors, cs.len(), *s as u8)
    })?;
    let card = pick_signal_card(weakest, false, config.attitude)?;
    Some((
        card,
        "discard-attitude",
        "Discarding from my weakest suit — a discouraging signal there.".to_string(),
    ))
}

fn longest_suit(cards: &[Card]) -> Option<Suit> {
    all_suits()
        .into_iter()
        .map(|s| (s, cards.iter().filter(|c| c.suit == s).count()))
        .filter(|(_, n)| *n >= 4)
        .max_by_key(|(s, n)| (*n, std::cmp::Reverse(*s as u8)))
        .map(|(s, _)| s)
}

// ── Defender: on lead mid-deal (§5.5) ──────────────────────────────────

fn defender_lead(ctx: &PlayContext, d: &Derived) -> Option<Fired> {
    // `return-partners-suit` (§5.5, first): top of remaining doubleton,
    // low from 3+.
    if let Some(suit) = d.partners_first_suit {
        let holding = suit_cards(&ctx.legal, suit);
        let card = match holding.len() {
            0 => None,
            1 | 2 => Some(holding[0]),
            _ => holding.last().copied(),
        };
        if let Some(card) = card {
            return Some((
                card,
                "return-partners-suit",
                "Returning partner's suit.".to_string(),
            ));
        }
    }

    // `cash-established-winner`: a sure trick is a sure trick.
    ctx.legal
        .iter()
        .copied()
        .filter(|c| Some(c.suit) != d.trump)
        .filter(|c| d.is_master(*c, &ctx.hand, true))
        .max_by_key(|c| c.rank)
        .map(|master| {
            (
                master,
                "cash-established-winner",
                "Cashing an established winner.".to_string(),
            )
        })
}

// ── Declarer side (§5.8, minimal) ──────────────────────────────────────

fn declarer_play(ctx: &PlayContext, d: &Derived) -> Option<Fired> {
    match d.position() {
        0 => {
            // Cash established winners when we have them.
            let master = ctx
                .legal
                .iter()
                .copied()
                .filter(|c| d.is_master(*c, &ctx.hand, true))
                .max_by_key(|c| c.rank)?;
            Some((
                master,
                "cash-established-winner",
                "Cashing an established winner.".to_string(),
            ))
        }
        _ => {
            if d.partner_is_winning() {
                return None; // partner (our other hand) has it — play cheap via fallback
            }
            let win = d.cheapest_winner(&ctx.legal)?;
            Some((
                win,
                "win-cheaply",
                "Winning the trick as cheaply as possible.".to_string(),
            ))
        }
    }
}

// ── Terminal rules ─────────────────────────────────────────────────────

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
