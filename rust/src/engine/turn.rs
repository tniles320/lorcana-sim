use super::events::{emit, Event};
use super::state::{draw_card, opponent_index, GameOver, GameOverReason, GameState, Phase};

fn ready_phase(state: &mut GameState) {
    let player = &mut state.players[state.active_player];
    for instance in player.play.iter_mut() {
        instance.exerted = false;
        instance.played_this_turn = false;
    }
    for ink in player.inkwell.iter_mut() {
        ink.exerted = false;
    }
    player.inked_this_turn = false;
}

fn set_phase(state: &mut GameState) {
    emit(&Event::StartOfTurn {
        active_player: state.active_player,
    });
}

fn draw_phase(state: &mut GameState) {
    let is_first_turn_of_game = state.turn_number == 1 && state.active_player == 0;
    if is_first_turn_of_game {
        return;
    }
    let active = state.active_player;
    if state.players[active].deck.is_empty() {
        state.game_over = Some(GameOver {
            winner: opponent_index(active),
            reason: GameOverReason::DeckOut,
        });
        return;
    }
    draw_card(&mut state.players[active]);
}

/// Runs Ready -> Set -> Draw for the active player and leaves state.phase at Main.
pub fn advance_to_main(state: &mut GameState) {
    state.phase = Phase::Ready;
    ready_phase(state);
    state.phase = Phase::Set;
    set_phase(state);
    state.phase = Phase::Draw;
    draw_phase(state);
    state.phase = Phase::Main;
}

/// Starts the whole game: runs turn 1's Ready/Set/Draw and lands on player 1's Main phase.
pub fn start_game(state: &mut GameState) {
    advance_to_main(state);
}

/// Ends the active player's turn and advances into the next player's Ready/Set/Draw/Main.
pub fn end_turn(state: &mut GameState) {
    emit(&Event::EndOfTurn {
        active_player: state.active_player,
    });
    state.active_player = if state.active_player == 0 { 1 } else { 0 };
    if state.active_player == 0 {
        state.turn_number += 1;
    }
    advance_to_main(state);
}
