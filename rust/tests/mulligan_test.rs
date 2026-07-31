mod common;

use common::CardBuilder;
use lorcana_sim::cards::Card;
use lorcana_sim::engine::state::{create_game, mulligan, system_rng};

fn deck_of(size: usize) -> Vec<Card> {
    (0..size).map(|_| CardBuilder::new().build()).collect()
}

#[test]
fn replaces_only_the_named_cards_and_keeps_hand_size_at_seven() {
    let mut rng = system_rng();
    let mut state = create_game(deck_of(60), deck_of(60), &mut rng);
    let player = &mut state.players[0];

    let kept_id = player.hand[0].instance_id.clone();
    let to_mulligan: Vec<String> = player.hand[1..4]
        .iter()
        .map(|c| c.instance_id.clone())
        .collect();

    mulligan(player, &to_mulligan, &mut rng);

    assert_eq!(player.hand.len(), 7);
    assert!(player.hand.iter().any(|c| c.instance_id == kept_id));
    for id in &to_mulligan {
        assert!(!player.hand.iter().any(|c| &c.instance_id == id));
    }
}

#[test]
fn deck_size_is_unchanged_after_mulligan() {
    let mut rng = system_rng();
    let mut state = create_game(deck_of(60), deck_of(60), &mut rng);
    let player = &mut state.players[0];
    let deck_size_before = player.deck.len();

    let to_mulligan: Vec<String> = player.hand[0..3]
        .iter()
        .map(|c| c.instance_id.clone())
        .collect();
    mulligan(player, &to_mulligan, &mut rng);

    // 3 drawn out, 3 shuffled back in -- net zero change.
    assert_eq!(player.deck.len(), deck_size_before);
}

#[test]
fn mulliganing_nothing_leaves_the_hand_untouched() {
    let mut rng = system_rng();
    let mut state = create_game(deck_of(60), deck_of(60), &mut rng);
    let player = &mut state.players[0];
    let hand_ids_before: Vec<String> = player.hand.iter().map(|c| c.instance_id.clone()).collect();

    mulligan(player, &[], &mut rng);

    let hand_ids_after: Vec<String> = player.hand.iter().map(|c| c.instance_id.clone()).collect();
    assert_eq!(hand_ids_before, hand_ids_after);
}
