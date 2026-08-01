mod common;

use common::CardBuilder;
use lorcana_sim::cards::load_deck;
use lorcana_sim::engine::state::{create_game, roll_for_first, system_rng};

fn deck_of(size: usize) -> Vec<lorcana_sim::cards::Card> {
    (0..size).map(|_| CardBuilder::new().build()).collect()
}

/// A simple LCG for reproducible test RNG. Uses u64 arithmetic (unlike the
/// TS test suite's inline LCG, which loses precision on large intermediate
/// products since JS numbers are f64) — this only needs to be internally
/// deterministic, not bit-identical to the TS version.
fn seeded_rng(seed: u64) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn shuffles_each_deck_draws_opening_hand_and_leaves_the_rest_in_the_deck() {
    let deck_a = deck_of(60);
    let deck_b = deck_of(60);
    let mut rng = lorcana_sim::engine::state::system_rng();
    let state = create_game(deck_a, deck_b, &mut rng);

    for player in &state.players {
        assert_eq!(player.hand.len(), 7);
        assert_eq!(player.deck.len(), 53);
        assert_eq!(player.play.len(), 0);
        assert_eq!(player.inkwell.len(), 0);
        assert_eq!(player.discard.len(), 0);
        assert_eq!(player.lore, 0);
    }
}

#[test]
fn is_deterministic_given_a_seeded_rng() {
    let deck_a = deck_of(10);
    let deck_b = deck_of(10);

    let mut rng_a = seeded_rng(42);
    let state_a = create_game(deck_a.clone(), deck_b.clone(), &mut rng_a);

    let mut rng_b = seeded_rng(42);
    let state_b = create_game(deck_a, deck_b, &mut rng_b);

    let names_a: Vec<&str> = state_a.players[0]
        .hand
        .iter()
        .map(|c| c.card.name.as_str())
        .collect();
    let names_b: Vec<&str> = state_b.players[0]
        .hand
        .iter()
        .map(|c| c.card.name.as_str())
        .collect();
    assert_eq!(names_a, names_b);
}

#[test]
fn loads_the_real_test_decks_and_builds_a_60_card_game() {
    let amber = load_deck("amber-vanilla-test.json");
    let steel = load_deck("steel-vanilla-test.json");
    assert_eq!(amber.len(), 60);
    assert_eq!(steel.len(), 60);

    let mut rng = lorcana_sim::engine::state::system_rng();
    let state = create_game(amber, steel, &mut rng);
    assert_eq!(state.players[0].hand.len(), 7);
    assert_eq!(state.players[1].hand.len(), 7);
}

#[test]
fn roll_for_first_eventually_returns_both_outcomes_and_terminates() {
    let mut rng = system_rng();
    let mut saw_true = false;
    let mut saw_false = false;
    for _ in 0..500 {
        if roll_for_first(&mut rng) {
            saw_true = true;
        } else {
            saw_false = true;
        }
        if saw_true && saw_false {
            break;
        }
    }
    assert!(saw_true && saw_false);
}
