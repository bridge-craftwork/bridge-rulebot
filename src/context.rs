//! Decision inputs.
//!
//! Contexts are owned snapshots, passed fresh on every call — the bot never
//! tracks play state between calls (see crate docs, "Statelessness"). The
//! shapes deliberately mirror the two existing seams: the solo client's
//! `cardplayBots.js` ctx object and the table service's folded table state,
//! so both adapters are thin translations rather than re-modelings.

use bridge_types::{Call, Card, Direction, Vulnerability};

/// One card from the play history, with the seat that played it.
///
/// The seat is derivable from the opening leader plus trick-winner logic,
/// but both callers already know it — passing it keeps the bot's derivation
/// code trivial and impossible to get subtly wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayedCard {
    /// Seat that played the card.
    pub seat: Direction,
    /// The card.
    pub card: Card,
}

/// Context for the opening lead (before dummy is faced).
#[derive(Debug, Clone)]
pub struct LeadContext {
    /// Seat on lead (always declarer's left-hand opponent).
    pub seat: Direction,
    /// The leader's 13 cards.
    pub hand: Vec<Card>,
    /// Declarer.
    pub declarer: Direction,
    /// Dealer (attributes auction calls to seats).
    pub dealer: Direction,
    /// Final contract, e.g. `"4S"`, `"3NT"`, `"2HX"`.
    pub contract: String,
    /// The auction, in call order starting from the dealer. Used by lead
    /// rules ("lead partner's bid suit"); pass empty if unavailable.
    pub auction: Vec<Call>,
    /// Board vulnerability.
    pub vulnerability: Vulnerability,
    /// Cards the bot may lead — for an opening lead this is the whole hand,
    /// but it stays an explicit field so the output contract ("the chosen
    /// card is always a member of `legal`") reads the same on both entry
    /// points.
    pub legal: Vec<Card>,
}

/// Context for any card after the opening lead.
#[derive(Debug, Clone)]
pub struct PlayContext {
    /// Seat to play. When the bot drives dummy's cards this is dummy's seat
    /// (the engine decides *whether* a bot controls dummy; the bot itself
    /// doesn't care).
    pub seat: Direction,
    /// The playing seat's *remaining* cards (original hand minus its entries
    /// in `played`).
    pub hand: Vec<Card>,
    /// Dummy's **original** 13 cards. The bot derives dummy's remaining
    /// cards from `played`. Always visible after the opening lead.
    pub dummy: Vec<Card>,
    /// Declarer (dummy is `declarer.partner()`).
    pub declarer: Direction,
    /// Dealer (attributes auction calls to seats).
    pub dealer: Direction,
    /// Final contract, e.g. `"4S"`, `"3NT"`, `"2HX"`.
    pub contract: String,
    /// The auction, in call order starting from the dealer; empty if
    /// unavailable.
    pub auction: Vec<Call>,
    /// Board vulnerability.
    pub vulnerability: Vulnerability,
    /// Every card played so far this deal, in chronological order across all
    /// completed and partial tricks. The current (incomplete) trick is the
    /// tail of this list; the bot reconstructs trick boundaries itself.
    pub played: Vec<PlayedCard>,
    /// The engine's pre-filtered legal subset of `hand`. The bot only ever
    /// returns a member of this set.
    pub legal: Vec<Card>,
}
