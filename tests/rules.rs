//! The worked examples from docs/requirements.md §5, near-verbatim.
//! Layout in every mid-play test: South declares, North is dummy, West led
//! the opening lead; East and West defend.

use bridge_rulebot::{
    choose_card, choose_opening_lead, AttitudeMethod, CountMethod, Decision, LeadContext,
    PlayContext, PlayedCard, SignalConfig,
};
use bridge_types::{Call, Card, Direction, Rank, Strain, Suit, Vulnerability};

fn card(s: &str) -> Card {
    let mut chars = s.chars();
    let suit = Suit::from_char(chars.next().unwrap()).unwrap();
    let rank = Rank::from_char(chars.next().unwrap()).unwrap();
    Card::new(suit, rank)
}

fn cards(s: &str) -> Vec<Card> {
    s.split_whitespace().map(card).collect()
}

fn pc(seat: Direction, c: &str) -> PlayedCard {
    PlayedCard {
        seat,
        card: card(c),
    }
}

/// Standard/standard config.
fn std() -> SignalConfig {
    SignalConfig::default()
}

fn config(attitude: AttitudeMethod, count: CountMethod) -> SignalConfig {
    SignalConfig { attitude, count }
}

/// Mid-play context: South declares `contract`, dummy North's ORIGINAL
/// cards in `dummy`, the bot plays `seat` holding `hand` (remaining), with
/// `legal` the engine-filtered subset.
fn play_ctx(
    seat: Direction,
    contract: &str,
    dummy: &str,
    hand: &str,
    legal: &str,
    played: Vec<PlayedCard>,
) -> PlayContext {
    PlayContext {
        seat,
        hand: cards(hand),
        dummy: cards(dummy),
        declarer: Direction::South,
        dealer: Direction::North,
        contract: contract.to_string(),
        auction: vec![],
        vulnerability: Vulnerability::None,
        played,
        legal: cards(legal),
    }
}

fn lead_ctx(hand: &str, contract: &str, auction: Vec<Call>) -> LeadContext {
    LeadContext {
        seat: Direction::West,
        hand: cards(hand),
        declarer: Direction::South,
        dealer: Direction::North,
        contract: contract.to_string(),
        auction,
        vulnerability: Vulnerability::None,
        legal: cards(hand),
    }
}

fn assert_pick(d: &Decision, want_card: &str, want_rule: &str) {
    assert_eq!(
        (d.card, d.rule),
        (card(want_card), want_rule),
        "explanation was: {}",
        d.explanation
    );
}

/// A completed first trick won by South, so South leads trick 2 (lets us
/// put a defender in second/fourth seat). Uses only club spots + the ♣A.
fn trick1_south_wins() -> Vec<PlayedCard> {
    use Direction::*;
    vec![
        pc(West, "C2"),
        pc(North, "C5"),
        pc(East, "C9"),
        pc(South, "CA"),
    ]
}

// ═══ §5.1 Opening leads ════════════════════════════════════════════════

#[test]
fn lead_top_of_sequence() {
    let ctx = lead_ctx("SK SQ SJ S5 S2 H7 H4 H3 D8 D6 D2 C9 C4", "3NT", vec![]);
    let d = choose_opening_lead(&ctx, &std()).unwrap();
    assert_pick(&d, "SK", "lead-top-of-sequence");
}

#[test]
fn lead_ace_from_ak_vs_suit() {
    let ctx = lead_ctx("HA HK H7 H3 D9 D6 D4 D2 C8 C5 C3 S2 S7", "4S", vec![]);
    let d = choose_opening_lead(&ctx, &std()).unwrap();
    assert_pick(&d, "HA", "lead-ace-from-ak");
}

#[test]
fn lead_fourth_best_vs_nt() {
    let ctx = lead_ctx("DQ D8 D6 D4 D2 S9 S7 S3 HK H5 C8 C6 C3", "3NT", vec![]);
    let d = choose_opening_lead(&ctx, &std()).unwrap();
    assert_pick(&d, "D4", "lead-fourth-best");
}

#[test]
fn lead_low_from_three_to_an_honor() {
    let ctx = lead_ctx("CK C7 C3 S9 S7 S5 S2 H8 H6 H2 D9 D4 D3", "3NT", vec![]);
    let d = choose_opening_lead(&ctx, &std()).unwrap();
    assert_pick(&d, "C3", "lead-low-from-honor");
}

#[test]
fn lead_top_of_nothing() {
    let ctx = lead_ctx("S8 S5 S2 H9 H7 H4 H3 D8 D6 D4 D3 C9 C7", "3NT", vec![]);
    let d = choose_opening_lead(&ctx, &std()).unwrap();
    assert_pick(&d, "S8", "lead-top-of-nothing");
}

#[test]
fn lead_partners_suit_low_from_honor() {
    // North dealt; East (partner of the West leader) overcalled hearts.
    let auction = vec![Call::Pass, Call::bid(1, Strain::Hearts)];
    let ctx = lead_ctx("HK H7 H3 S9 S6 S4 S2 D8 D5 D3 C9 C6 C2", "4S", auction);
    let d = choose_opening_lead(&ctx, &std()).unwrap();
    assert_pick(&d, "H3", "lead-partners-suit");
}

#[test]
fn never_underleads_an_ace_vs_a_suit_contract() {
    let ctx = lead_ctx("HA H7 H4 H2 S8 S6 S3 D9 D5 D2 CJ C4 C8", "4S", vec![]);
    let d = choose_opening_lead(&ctx, &std()).unwrap();
    assert_ne!(d.card, card("H2"), "underled an ace vs a suit contract");
    assert_ne!(d.card, card("H4"), "underled an ace vs a suit contract");
}

// ═══ §5.2 Second hand ══════════════════════════════════════════════════

#[test]
fn second_hand_low() {
    // South leads the ♦3; West holds ♦K84.
    let mut played = trick1_south_wins();
    played.push(pc(Direction::South, "D3"));
    let ctx = play_ctx(
        Direction::West,
        "3NT",
        "S7 S4 H8 H3 D9 C5",
        "DK D8 D4 S9 H6",
        "DK D8 D4",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "D4", "second-hand-low");
}

#[test]
fn second_hand_covers_an_honor() {
    // South leads the ♠J; West holds ♠Q63.
    let mut played = trick1_south_wins();
    played.push(pc(Direction::South, "SJ"));
    let ctx = play_ctx(
        Direction::West,
        "3NT",
        "S7 S4 H8 H3 D9 C5",
        "SQ S6 S3 H6 D2",
        "SQ S6 S3",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "SQ", "second-hand-cover-honor");
}

#[test]
fn second_hand_splits_touching_honors() {
    // South leads the ♥2; West holds ♥KQ7.
    let mut played = trick1_south_wins();
    played.push(pc(Direction::South, "H2"));
    let ctx = play_ctx(
        Direction::West,
        "3NT",
        "S7 S4 H8 H3 D9 C6",
        "HK HQ H7 D5 C4",
        "HK HQ H7",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "HQ", "second-hand-split-honors");
}

// ═══ §5.3 Third hand ═══════════════════════════════════════════════════

#[test]
fn third_hand_high() {
    // Partner (West) leads the ♠5, dummy plays low; East holds ♠K82.
    let played = vec![pc(Direction::West, "S5"), pc(Direction::North, "S3")];
    let ctx = play_ctx(
        Direction::East,
        "3NT",
        "S7 S4 S3 H9 H6 D8",
        "SK S8 S2 H5 D4",
        "SK S8 S2",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "SK", "third-hand-high");
}

#[test]
fn third_hand_only_as_high_as_needed() {
    // Dummy holds ♠Q94 and plays the ♠4; East holds ♠J105 — the ♠10 does
    // the job because dummy's ♠9 is visible.
    let played = vec![pc(Direction::West, "S2"), pc(Direction::North, "S4")];
    let ctx = play_ctx(
        Direction::East,
        "3NT",
        "SQ S9 S4 H8 H3 D6",
        "SJ ST S5 H7 D2",
        "SJ ST S5",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "ST", "third-hand-only-as-high-as-needed");
}

#[test]
fn third_hand_unblocks_doubleton_honor() {
    // Partner leads the ♦K (top of a sequence); East holds ♦J3.
    let played = vec![pc(Direction::West, "DK"), pc(Direction::North, "D2")];
    let ctx = play_ctx(
        Direction::East,
        "3NT",
        "D7 D2 S8 S5 H9 H4",
        "DJ D3 S9 H6 C3",
        "DJ D3",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "DJ", "third-hand-unblock");
}

// ═══ §4 Attitude matrix (all four combinations) ════════════════════════

fn attitude_ctx(holding: &str) -> PlayContext {
    // Partner leads the ♠K (winning honor); dummy plays low; East signals.
    let played = vec![pc(Direction::West, "SK"), pc(Direction::North, "S4")];
    play_ctx(
        Direction::East,
        "3NT",
        "S7 S4 H8 H3 D9 C5",
        holding,
        holding,
        played,
    )
}

#[test]
fn attitude_matrix() {
    use AttitudeMethod as A;
    use CountMethod as C;
    // (attitude, count, holding, expected card, expected rule)
    let cases = [
        (
            A::Standard,
            C::Standard,
            "SQ S9 S2",
            "S9",
            "attitude-encourage",
        ),
        (
            A::Standard,
            C::UpsideDown,
            "SQ S9 S2",
            "S9",
            "attitude-encourage",
        ),
        (
            A::UpsideDown,
            C::Standard,
            "SQ S9 S2",
            "S2",
            "attitude-encourage",
        ),
        (
            A::UpsideDown,
            C::UpsideDown,
            "SQ S9 S2",
            "S2",
            "attitude-encourage",
        ),
        (
            A::Standard,
            C::Standard,
            "S9 S7 S2",
            "S2",
            "attitude-discourage",
        ),
        (
            A::Standard,
            C::UpsideDown,
            "S9 S7 S2",
            "S2",
            "attitude-discourage",
        ),
        (
            A::UpsideDown,
            C::Standard,
            "S9 S7 S2",
            "S9",
            "attitude-discourage",
        ),
        (
            A::UpsideDown,
            C::UpsideDown,
            "S9 S7 S2",
            "S9",
            "attitude-discourage",
        ),
    ];
    for (att, cnt, holding, want, rule) in cases {
        let d = choose_card(&attitude_ctx(holding), &config(att, cnt)).unwrap();
        assert_pick(&d, want, rule);
    }
}

// ═══ §4 Count matrix (all four combinations) ═══════════════════════════

fn count_ctx(holding: &str) -> PlayContext {
    // Declarer (South) leads the ♦3 toward dummy; West follows with spots
    // only — the card is free, give count.
    let mut played = trick1_south_wins();
    played.push(pc(Direction::South, "D3"));
    play_ctx(
        Direction::West,
        "3NT",
        "DA DK DQ H8 H3 C6",
        holding,
        holding,
        played,
    )
}

#[test]
fn count_matrix() {
    use AttitudeMethod as A;
    use CountMethod as C;
    let cases = [
        // ♦84 = even: standard starts high-low, upside-down starts low.
        (A::Standard, C::Standard, "D8 D4", "D8"),
        (A::UpsideDown, C::Standard, "D8 D4", "D8"),
        (A::Standard, C::UpsideDown, "D8 D4", "D4"),
        (A::UpsideDown, C::UpsideDown, "D8 D4", "D4"),
        // ♦852 = odd: standard starts low, upside-down starts high.
        (A::Standard, C::Standard, "D8 D5 D2", "D2"),
        (A::UpsideDown, C::Standard, "D8 D5 D2", "D2"),
        (A::Standard, C::UpsideDown, "D8 D5 D2", "D8"),
        (A::UpsideDown, C::UpsideDown, "D8 D5 D2", "D8"),
    ];
    for (att, cnt, holding, want) in cases {
        let d = choose_card(&count_ctx(holding), &config(att, cnt)).unwrap();
        assert_pick(&d, want, "count-signal");
    }
}

// ═══ §5.4 Fourth hand ══════════════════════════════════════════════════

#[test]
fn fourth_hand_wins_cheaply() {
    use Direction::*;
    // North won trick 1 and leads the ♠3; South's ♠Q is winning when
    // West (4th) holds ♠AK4 — the king is the cheapest sufficient card.
    let played = vec![
        pc(West, "C2"),
        pc(North, "CA"),
        pc(East, "C4"),
        pc(South, "C6"),
        pc(North, "S3"),
        pc(East, "S5"),
        pc(South, "SQ"),
    ];
    let ctx = play_ctx(
        West,
        "3NT",
        "S3 H8 H5 D9 D6 CA",
        "SA SK S4 H7 D2",
        "SA SK S4",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "SK", "win-cheaply");
}

// ═══ §5.6 Ruffs ════════════════════════════════════════════════════════

#[test]
fn ruffs_to_win_with_cheapest_trump() {
    // 4♠ by South. Hearts led, East is void holding ♠74 of trumps; the
    // opponents hold the trick — ruff with the ♠4.
    let played = vec![pc(Direction::West, "H2"), pc(Direction::North, "H5")];
    let ctx = play_ctx(
        Direction::East,
        "4S",
        "H5 H8 D7 D4 C8 C3",
        "S7 S4 D9 D3 C6",
        "S7 S4 D9 D3 C6",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "S4", "ruff-to-win");
}

#[test]
fn overruffs_cheaply() {
    use Direction::*;
    // South ruffed with the ♠6; West overruffs with the ♠8 from ♠J8.
    let played = vec![
        pc(West, "C3"),
        pc(North, "CA"),
        pc(East, "C4"),
        pc(South, "C9"),
        pc(North, "D4"),
        pc(East, "D7"),
        pc(South, "S6"),
    ];
    let ctx = play_ctx(
        West,
        "4S",
        "D4 DQ H8 H5 CA C7",
        "SJ S8 C5 C2 H9",
        "SJ S8 C5 C2 H9",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "S8", "overruff-cheaply");
}

#[test]
fn never_ruffs_partners_winner() {
    // Partner's ♥A is winning; East is void with trumps but must not
    // waste one — discard instead.
    let played = vec![pc(Direction::West, "HA"), pc(Direction::North, "H6")];
    let ctx = play_ctx(
        Direction::East,
        "4S",
        "H6 H9 D8 D5 C7 C2",
        "S8 S5 DJ D7 D4 D2 C9 C3",
        "S8 S5 DJ D7 D4 D2 C9 C3",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_ne!(d.card.suit, Suit::Spades, "ruffed partner's winner");
    assert_eq!(d.rule, "discard-attitude");
}

// ═══ §5.7 Discards ═════════════════════════════════════════════════════

#[test]
fn discard_attitude_matrix() {
    use AttitudeMethod::*;
    // East can't follow the diamond lead; holding ♥Q1085 and wanting
    // hearts: pitch the ♥8 (standard) or a low heart (upside-down).
    let mut played = trick1_south_wins();
    played.extend([
        pc(Direction::South, "DK"),
        pc(Direction::West, "D5"),
        pc(Direction::North, "D8"),
    ]);
    let make = || {
        play_ctx(
            Direction::East,
            "3NT",
            "D8 D9 S6 S4 H7 C8",
            "HQ HT H8 H5 C9 C4 S7 S3",
            "HQ HT H8 H5 C9 C4 S7 S3",
            played.clone(),
        )
    };
    let d = choose_card(&make(), &config(Standard, CountMethod::Standard)).unwrap();
    assert_pick(&d, "H8", "discard-attitude");
    let d = choose_card(&make(), &config(UpsideDown, CountMethod::Standard)).unwrap();
    assert_pick(&d, "H5", "discard-attitude");
}

// ═══ §5.5 Defender continuation ════════════════════════════════════════

#[test]
fn returns_partners_suit_top_of_doubleton() {
    use Direction::*;
    // West led hearts; East won the trick with the ♥A and returns the ♥8
    // from the remaining ♥86.
    let played = vec![
        pc(West, "H4"),
        pc(North, "H2"),
        pc(East, "HA"),
        pc(South, "H6"),
    ];
    let ctx = play_ctx(
        East,
        "3NT",
        "H2 H9 S8 S5 D7 C6",
        "H8 H6 S9 S4 D3",
        "H8 H6 S9 S4 D3",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "H8", "return-partners-suit");
}

#[test]
fn cashes_established_winner_on_lead() {
    use Direction::*;
    // East won trick 1 with the ♦Q (partner led diamonds but East has none
    // left) and holds the ♠A — cash it.
    let played = vec![
        pc(West, "D2"),
        pc(North, "D9"),
        pc(East, "DQ"),
        pc(South, "D3"),
    ];
    let ctx = play_ctx(
        East,
        "3NT",
        "D9 D8 S6 S3 H7 C5",
        "SA C7 C4 H9 H2",
        "SA C7 C4 H9 H2",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "SA", "cash-established-winner");
}

// ═══ §5.8 Declarer (minimal) ═══════════════════════════════════════════

#[test]
fn declarer_wins_cheaply_in_fourth_seat() {
    use Direction::*;
    // West leads the ♠Q; dummy and East play low; South wins with the
    // king, not the ace.
    let played = vec![pc(West, "SQ"), pc(North, "S2"), pc(East, "S5")];
    let ctx = play_ctx(
        South,
        "3NT",
        "S2 S7 H8 H4 D6 C9",
        "SA SK S4 H6 D3",
        "SA SK S4",
        played,
    );
    let d = choose_card(&ctx, &std()).unwrap();
    assert_pick(&d, "SK", "win-cheaply");
}
