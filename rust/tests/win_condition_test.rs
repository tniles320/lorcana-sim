mod common;

use common::CardBuilder;
use lorcana_sim::cards::Card;
use lorcana_sim::engine::events::clear_handlers;
use lorcana_sim::engine::state::{
    check_lore_victory, create_game, system_rng, GameOverReason, GameState, LORE_TO_WIN,
};
use lorcana_sim::engine::turn::{end_turn, start_game};

fn deck_of(size: usize) -> Vec<Card> {
    (0..size).map(|_| CardBuilder::new().build()).collect()
}

fn new_game() -> GameState {
    clear_handlers();
    let mut rng = system_rng();
    create_game(deck_of(60), deck_of(60), &mut rng)
}

#[test]
fn no_winner_below_the_lore_threshold() {
    let mut state = new_game();
    state.players[0].lore = LORE_TO_WIN - 1;
    check_lore_victory(&mut state);
    assert!(state.game_over.is_none());
}

#[test]
fn player_reaching_the_lore_threshold_wins() {
    let mut state = new_game();
    state.players[1].lore = LORE_TO_WIN;
    check_lore_victory(&mut state);
    let over = state.game_over.expect("game should be over");
    assert_eq!(over.winner, 1);
    assert_eq!(over.reason, GameOverReason::LoreVictory);
}

#[test]
fn is_idempotent_once_the_game_has_ended() {
    let mut state = new_game();
    state.players[0].lore = LORE_TO_WIN;
    check_lore_victory(&mut state);
    // Player 2 also crosses the threshold after the fact -- shouldn't
    // overwrite the already-recorded winner.
    state.players[1].lore = LORE_TO_WIN + 5;
    check_lore_victory(&mut state);
    assert_eq!(state.game_over.unwrap().winner, 0);
}

#[test]
fn a_player_forced_to_draw_from_an_empty_deck_loses() {
    let mut state = new_game();
    start_game(&mut state);
    // Player 1's deck is empty; ending the turn moves to player 2 first
    // (whose deck is untouched), then back to player 1, who must draw.
    state.players[0].deck.clear();
    end_turn(&mut state); // -> player 2's turn, draws fine
    assert!(state.game_over.is_none());
    end_turn(&mut state); // -> player 1's turn, deck is empty, must draw
    let over = state.game_over.expect("player 1 should have decked out");
    assert_eq!(over.winner, 1);
    assert_eq!(over.reason, GameOverReason::DeckOut);
}
