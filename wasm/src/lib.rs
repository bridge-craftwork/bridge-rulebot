//! Browser (wasm) boundary for `bridge-rulebot`.
//!
//! The pure core takes owned `LeadContext` / `PlayContext` built from
//! `bridge_types`. This crate exposes two `wasm_bindgen` entry points that
//! accept a **JSON string** (built by the frontend's `cardplayBots.js` adapter
//! from its bot `ctx`), deserialize it into JS-friendly DTOs, convert those to
//! the core types, run the core, and return the chosen card + reason as JSON.
//!
//! The JSON boundary (rather than `serde-wasm-bindgen`) keeps the contract
//! explicit and debuggable, and keeps the core crates free of serde.
//!
//! Card shape on the wire mirrors the frontend's `{ suit, rank }` objects
//! (e.g. `{ "suit": "S", "rank": "A" }`); auction calls are PBN strings
//! (`"1N"`, `"Pass"`, `"X"`); seats/vulnerability are the app's usual codes.

use bridge_rulebot::{choose_card, choose_opening_lead, LeadContext, PlayContext, PlayedCard};
use bridge_rulebot::{AttitudeMethod, CountMethod, SignalConfig};
use bridge_types::{Call, Card, Direction, Rank, Suit, Vulnerability};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ── Wire DTOs (JS → core) ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct CardDto {
    suit: String,
    rank: String,
}

#[derive(Deserialize)]
struct PlayedDto {
    seat: String,
    suit: String,
    rank: String,
}

#[derive(Deserialize)]
struct LeadCtxDto {
    seat: String,
    hand: Vec<CardDto>,
    declarer: String,
    dealer: String,
    contract: String,
    /// Auction calls in bidding order from the dealer, as PBN strings.
    auction: Vec<String>,
    vulnerable: String,
    /// The leader's full remaining hand (any card may be led).
    legal: Vec<CardDto>,
}

#[derive(Deserialize)]
struct PlayCtxDto {
    seat: String,
    hand: Vec<CardDto>,
    dummy: Vec<CardDto>,
    declarer: String,
    dealer: String,
    contract: String,
    auction: Vec<String>,
    vulnerable: String,
    /// Every card played so far, chronological, all seats.
    played: Vec<PlayedDto>,
    /// The engine's pre-filtered legal subset for this turn.
    legal: Vec<CardDto>,
}

#[derive(Default, Deserialize)]
struct ConfigDto {
    /// "standard" | "upside_down" (default standard).
    #[serde(default)]
    attitude: Option<String>,
    #[serde(default)]
    count: Option<String>,
}

// ── Result DTO (core → JS) ─────────────────────────────────────────────────

#[derive(Serialize)]
struct DecisionDto {
    suit: String,
    rank: String,
    /// The rule slug that fired (telemetry/UI key — stable API).
    rule: String,
    /// Student-facing explanation (wording not an API contract).
    explanation: String,
}

// ── Conversions ────────────────────────────────────────────────────────────

fn parse_char(s: &str) -> Result<char, String> {
    s.chars().next().ok_or_else(|| format!("empty code: {s:?}"))
}

fn to_dir(s: &str) -> Result<Direction, String> {
    Direction::from_char(parse_char(s)?).ok_or_else(|| format!("bad seat: {s:?}"))
}

fn to_card(c: &CardDto) -> Result<Card, String> {
    let suit =
        Suit::from_char(parse_char(&c.suit)?).ok_or_else(|| format!("bad suit: {:?}", c.suit))?;
    let rank =
        Rank::from_char(parse_char(&c.rank)?).ok_or_else(|| format!("bad rank: {:?}", c.rank))?;
    Ok(Card { suit, rank })
}

fn to_cards(v: &[CardDto]) -> Result<Vec<Card>, String> {
    v.iter().map(to_card).collect()
}

fn to_auction(v: &[String]) -> Result<Vec<Call>, String> {
    v.iter()
        .map(|s| Call::from_pbn(s).ok_or_else(|| format!("bad call: {s:?}")))
        .collect()
}

fn to_vuln(s: &str) -> Result<Vulnerability, String> {
    // The app passes 'None' | 'NS' | 'EW' | 'All'; from_pbn maps all of them.
    Vulnerability::from_pbn(s).ok_or_else(|| format!("bad vulnerability: {s:?}"))
}

fn to_config(dto: &ConfigDto) -> SignalConfig {
    let attitude = match dto.attitude.as_deref() {
        Some("upside_down") | Some("udca") => AttitudeMethod::UpsideDown,
        _ => AttitudeMethod::Standard,
    };
    let count = match dto.count.as_deref() {
        Some("upside_down") => CountMethod::UpsideDown,
        _ => CountMethod::Standard,
    };
    SignalConfig { attitude, count }
}

fn config_from_json(config_json: &str) -> SignalConfig {
    // A missing/invalid config is not an error — default to standard/standard.
    serde_json::from_str::<ConfigDto>(config_json)
        .map(|d| to_config(&d))
        .unwrap_or_default()
}

fn decision_json(card: Card, rule: &str, explanation: String) -> String {
    let dto = DecisionDto {
        suit: card.suit.to_char().to_string(),
        rank: card.rank.to_char().to_string(),
        rule: rule.to_string(),
        explanation,
    };
    // Serializing a fixed-shape struct cannot fail.
    serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string())
}

// ── Entry points ───────────────────────────────────────────────────────────

/// One-time init: nicer panic messages in the browser console. Optional.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Choose an opening lead. `ctx_json` is a `LeadCtxDto`; `config_json` a
/// `ConfigDto` (may be `"{}"`). Returns a `DecisionDto` JSON, or throws a
/// string error (empty legal set / malformed input).
#[wasm_bindgen]
pub fn choose_opening_lead_json(ctx_json: &str, config_json: &str) -> Result<String, String> {
    let dto: LeadCtxDto = serde_json::from_str(ctx_json).map_err(|e| e.to_string())?;
    let ctx = LeadContext {
        seat: to_dir(&dto.seat)?,
        hand: to_cards(&dto.hand)?,
        declarer: to_dir(&dto.declarer)?,
        dealer: to_dir(&dto.dealer)?,
        contract: dto.contract,
        auction: to_auction(&dto.auction)?,
        vulnerability: to_vuln(&dto.vulnerable)?,
        legal: to_cards(&dto.legal)?,
    };
    let d =
        choose_opening_lead(&ctx, &config_from_json(config_json)).map_err(|e| format!("{e:?}"))?;
    Ok(decision_json(d.card, d.rule, d.explanation))
}

/// Choose a card mid-play. `ctx_json` is a `PlayCtxDto`.
#[wasm_bindgen]
pub fn choose_card_json(ctx_json: &str, config_json: &str) -> Result<String, String> {
    let dto: PlayCtxDto = serde_json::from_str(ctx_json).map_err(|e| e.to_string())?;
    let ctx = PlayContext {
        seat: to_dir(&dto.seat)?,
        hand: to_cards(&dto.hand)?,
        dummy: to_cards(&dto.dummy)?,
        declarer: to_dir(&dto.declarer)?,
        dealer: to_dir(&dto.dealer)?,
        contract: dto.contract,
        auction: to_auction(&dto.auction)?,
        vulnerability: to_vuln(&dto.vulnerable)?,
        played: dto
            .played
            .iter()
            .map(|p| {
                Ok::<_, String>(PlayedCard {
                    seat: to_dir(&p.seat)?,
                    card: to_card(&CardDto {
                        suit: p.suit.clone(),
                        rank: p.rank.clone(),
                    })?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        legal: to_cards(&dto.legal)?,
    };
    let d = choose_card(&ctx, &config_from_json(config_json)).map_err(|e| format!("{e:?}"))?;
    Ok(decision_json(d.card, d.rule, d.explanation))
}
