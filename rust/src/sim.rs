//! Phase 6 of the scope doc, minimal version: play a game to completion
//! with the heuristic bot on both sides, silently (no per-move logging --
//! see examples/full_game.rs for that). Exists so a batch runner can play
//! many games without re-deriving the core loop each time.

use crate::bot::{choose_move, decide_mulligan};
use crate::cards::Card;
use crate::engine::actions::apply_move;
use crate::engine::state::{check_lore_victory, create_game, mulligan, GameOver};
use crate::engine::turn::{end_turn, start_game};

/// Backstop against an actual infinite-loop bug -- a real game with these
/// decks should finish well under this (~53 turns per player before either
/// side decks out, if it hasn't ended on lore first).
pub const MAX_TURNS: u32 = 300;

#[derive(Debug, Clone, Copy)]
pub struct GameResult {
    /// `None` means the MAX_TURNS safety valve was hit with no winner --
    /// shouldn't happen in practice, but distinct from a real result.
    pub game_over: Option<GameOver>,
    pub turn_number: u32,
    pub final_lore: [i32; 2],
}

pub fn play_game(
    deck_a: Vec<Card>,
    deck_b: Vec<Card>,
    rng: &mut impl FnMut() -> f64,
) -> GameResult {
    let mut state = create_game(deck_a, deck_b, rng);
    for player in state.players.iter_mut() {
        let to_mulligan = decide_mulligan(&player.hand);
        mulligan(player, &to_mulligan, rng);
    }
    start_game(&mut state);

    let mut turn_count: u32 = 0;

    loop {
        check_lore_victory(&mut state);
        if state.game_over.is_some() {
            break;
        }

        loop {
            let mv = choose_move(&state);
            let is_pass = matches!(mv, crate::engine::actions::Move::Pass);
            apply_move(&mut state, &mv).expect("bot only ever chooses legal moves");
            check_lore_victory(&mut state);
            if is_pass || state.game_over.is_some() {
                break;
            }
        }

        if state.game_over.is_some() {
            break;
        }

        end_turn(&mut state);
        turn_count += 1;
        if turn_count > MAX_TURNS {
            break;
        }
    }

    GameResult {
        game_over: state.game_over,
        turn_number: state.turn_number,
        final_lore: [state.players[0].lore, state.players[1].lore],
    }
}
