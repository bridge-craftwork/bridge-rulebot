//! State derived per call from `(context, played)`.
//!
//! The bot is stateless (see crate docs), so everything a rule wants to know
//! beyond the raw context — trick boundaries, who's winning the current
//! trick, dummy's remaining cards, which ranks are still unseen — is
//! recomputed here on every call. At 52 cards maximum this is trivially
//! cheap, and it means rules read like the guideline they implement.

use crate::context::{PlayContext, PlayedCard};
use bridge_types::{Card, Direction, Rank, Suit};

/// One reconstructed trick.
#[derive(Debug, Clone)]
pub(crate) struct TrickView {
    pub leader: Direction,
    /// In play order; complete tricks have 4 entries.
    pub plays: Vec<PlayedCard>,
}

impl TrickView {
    pub fn led_suit(&self) -> Option<Suit> {
        self.plays.first().map(|p| p.card.suit)
    }

    /// Who currently holds the trick, given the trump suit.
    pub fn winner(&self, trump: Option<Suit>) -> Option<PlayedCard> {
        let led = self.led_suit()?;
        self.plays
            .iter()
            .copied()
            .max_by_key(|p| trick_power(p.card, led, trump))
    }
}

/// Ordering key for cards within a trick: trumps beat the led suit, the led
/// suit beats discards, rank breaks ties within a class.
fn trick_power(card: Card, led: Suit, trump: Option<Suit>) -> (u8, u8) {
    let class = if Some(card.suit) == trump {
        2
    } else if card.suit == led {
        1
    } else {
        0
    };
    (class, card.rank as u8)
}

/// Everything precomputed for the mid-play rules.
pub(crate) struct Derived {
    pub trump: Option<Suit>,
    /// The trick in progress (never complete; the engine only asks for a
    /// card when one is owed). `None` only for a malformed empty history —
    /// callers treat that as "we are leading".
    pub current: Option<TrickView>,
    /// Seat's partner.
    pub partner: Direction,
    /// True when the playing seat defends (not declarer's side).
    pub is_defender: bool,
    /// Dummy's *remaining* cards (original minus played).
    pub dummy_remaining: Vec<Card>,
    /// The suit the bot's partner first led this deal, if any.
    pub partners_first_suit: Option<Suit>,
    /// All cards played so far (chronological).
    pub played: Vec<PlayedCard>,
}

impl Derived {
    pub fn compute(ctx: &PlayContext) -> Derived {
        let trump = trump_suit(&ctx.contract);
        let opening_leader = ctx.declarer.next();
        let tricks = reconstruct_tricks(&ctx.played, opening_leader, trump);
        let current = tricks.iter().last().filter(|t| t.plays.len() < 4).cloned();

        let dummy_seat = ctx.declarer.partner();
        let dummy_remaining: Vec<Card> = ctx
            .dummy
            .iter()
            .copied()
            .filter(|c| !ctx.played.iter().any(|p| p.card == *c))
            .collect();

        let partner = ctx.seat.partner();
        let is_defender = ctx.seat != ctx.declarer && ctx.seat != dummy_seat;
        // Only leads count as "partner's suit", not follows.
        let partners_first_suit = tricks
            .iter()
            .find(|t| t.leader == partner)
            .and_then(|t| t.led_suit());

        Derived {
            trump,
            current,
            partner,
            is_defender,
            dummy_remaining,
            partners_first_suit,
            played: ctx.played.clone(),
        }
    }

    /// 0 = leading, 1 = second hand, 2 = third hand, 3 = fourth hand.
    pub fn position(&self) -> usize {
        self.current.as_ref().map_or(0, |t| t.plays.len())
    }

    /// The card/seat currently winning the trick in progress.
    pub fn winning(&self) -> Option<PlayedCard> {
        self.current.as_ref().and_then(|t| t.winner(self.trump))
    }

    /// Whether the bot's partner currently holds the trick.
    pub fn partner_is_winning(&self) -> bool {
        self.winning().is_some_and(|w| w.seat == self.partner)
    }

    /// The cheapest card in `cards` that would take over the current trick
    /// (beats the winning card, honoring trump/led-suit classes).
    pub fn cheapest_winner(&self, cards: &[Card]) -> Option<Card> {
        let trick = self.current.as_ref()?;
        let led = trick.led_suit()?;
        let win_power = trick_power(self.winning()?.card, led, self.trump);
        cards
            .iter()
            .copied()
            .filter(|c| trick_power(*c, led, self.trump) > win_power)
            .min_by_key(|c| (c.rank, c.suit))
    }

    /// True when `card` is the highest still-unseen-or-held card of its suit
    /// from this seat's point of view: nothing above it remains in unseen
    /// space or dummy (i.e. it can't be beaten in its suit).
    pub fn is_master(&self, card: Card, hand: &[Card], dummy_visible: bool) -> bool {
        let higher_unaccounted = all_ranks_above(card.rank).into_iter().any(|r| {
            let c = Card::new(card.suit, r);
            let seen_played = self.played.iter().any(|p| p.card == c);
            let in_hand = hand.contains(&c);
            let in_dummy = dummy_visible && self.dummy_remaining.contains(&c);
            !(seen_played || in_hand || in_dummy)
        });
        !higher_unaccounted
    }

    /// Collapse `cards` (same suit, sorted desc) into the top equal-group:
    /// cards are equals when every rank between them is accounted for
    /// (played, in this hand, or visible in dummy). Returns the *lowest*
    /// card of the top group — "as high as needed, as cheap as possible".
    pub fn bottom_of_top_equals(&self, suit_cards: &[Card], hand: &[Card]) -> Option<Card> {
        let mut sorted: Vec<Card> = suit_cards.to_vec();
        sorted.sort_by_key(|c| std::cmp::Reverse(c.rank));
        let mut best = *sorted.first()?;
        for c in sorted.iter().skip(1) {
            if self.ranks_between_accounted(best.suit, c.rank, best.rank, hand) {
                best = *c;
            } else {
                break;
            }
        }
        Some(best)
    }

    /// Every rank strictly between `lo` and `hi` in `suit` is played, in
    /// this hand, or visible in dummy.
    fn ranks_between_accounted(&self, suit: Suit, lo: Rank, hi: Rank, hand: &[Card]) -> bool {
        all_ranks()
            .into_iter()
            .filter(|r| *r > lo && *r < hi)
            .all(|r| {
                let c = Card::new(suit, r);
                self.played.iter().any(|p| p.card == c)
                    || hand.contains(&c)
                    || self.dummy_remaining.contains(&c)
            })
    }
}

/// Trump suit from a contract string like "4S", "3NT", "2HX".
pub(crate) fn trump_suit(contract: &str) -> Option<Suit> {
    contract
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .chars()
        .next()
        .and_then(Suit::from_char)
}

/// Rebuild trick boundaries from the flat chronological history.
pub(crate) fn reconstruct_tricks(
    played: &[PlayedCard],
    opening_leader: Direction,
    trump: Option<Suit>,
) -> Vec<TrickView> {
    let mut tricks: Vec<TrickView> = Vec::new();
    let mut leader = opening_leader;
    for chunk in played.chunks(4) {
        let t = TrickView {
            leader,
            plays: chunk.to_vec(),
        };
        if chunk.len() == 4 {
            if let Some(w) = t.winner(trump) {
                leader = w.seat;
            }
        }
        tricks.push(t);
    }
    tricks
}

/// A "spot" card for signaling purposes: nine or below.
pub(crate) fn is_spot(card: Card) -> bool {
    card.rank <= Rank::Nine
}

/// An honor: ten or above. (Tens count — they matter in sequences and are
/// too valuable to burn as signals.)
pub(crate) fn is_honor(card: Card) -> bool {
    card.rank >= Rank::Ten
}

fn all_ranks() -> [Rank; 13] {
    use Rank::*;
    [
        Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace,
    ]
}

fn all_ranks_above(rank: Rank) -> Vec<Rank> {
    all_ranks().into_iter().filter(|r| *r > rank).collect()
}
