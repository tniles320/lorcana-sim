mod common;

use common::CardBuilder;
use lorcana_sim::cards::Card;
use lorcana_sim::engine::events::clear_handlers;
use lorcana_sim::engine::state::{create_game, system_rng, GameState, Phase};
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
fn start_game_does_not_draw_for_the_first_players_first_turn() {
    let mut state = new_game();
    start_game(&mut state);
    assert_eq!(state.phase, Phase::Main);
    assert_eq!(state.players[0].hand.len(), 7);
}

#[test]
fn end_turn_draws_for_the_second_players_first_turn() {
    let mut state = new_game();
    start_game(&mut state);
    end_turn(&mut state);
    assert_eq!(state.active_player, 1);
    assert_eq!(state.players[1].hand.len(), 8);
    assert_eq!(state.turn_number, 1);
}

#[test]
fn turn_number_increments_only_when_wrapping_back_to_player_0() {
    let mut state = new_game();
    start_game(&mut state);
    end_turn(&mut state);
    assert_eq!(state.turn_number, 1);
    end_turn(&mut state);
    assert_eq!(state.active_player, 0);
    assert_eq!(state.turn_number, 2);
}

#[test]
fn ready_phase_only_resets_flags_on_that_players_own_turn() {
    let mut state = new_game();
    start_game(&mut state);

    let mut character = state.players[0].hand.pop().unwrap();
    character.exerted = true;
    character.played_this_turn = true;
    let character_id = character.instance_id.clone();
    state.players[0].play.push(character);

    let mut ink = state.players[0].hand.pop().unwrap();
    ink.exerted = true;
    state.players[0].inkwell.push(ink);
    state.players[0].inked_this_turn = true;

    end_turn(&mut state); // -> player 2's turn; player 1's flags untouched
    let still_exerted = state.players[0]
        .play
        .iter()
        .find(|c| c.instance_id == character_id)
        .unwrap();
    assert!(still_exerted.exerted);
    assert!(state.players[0].inked_this_turn);

    end_turn(&mut state); // -> back to player 1's turn; Ready phase runs for player 1
    let reset = state.players[0]
        .play
        .iter()
        .find(|c| c.instance_id == character_id)
        .unwrap();
    assert!(!reset.exerted);
    assert!(!reset.played_this_turn);
    assert!(state.players[0].inkwell.iter().all(|c| !c.exerted));
    assert!(!state.players[0].inked_this_turn);
}
