//! Plays one full game between the two vanilla-heavy test decks, using the
//! crude heuristic bot for both sides, and prints a readable move-by-move
//! log. This is the first time the engine runs to actual completion instead
//! of a hand-scripted scenario.
//!
//! Run: cargo run --example full_game

use lorcana_sim::bot::choose_move;
use lorcana_sim::cards::load_deck;
use lorcana_sim::engine::actions::{apply_move, Move};
use lorcana_sim::engine::state::{
    check_lore_victory, create_game, opponent_index, system_rng, CardInstance, GameState,
};
use lorcana_sim::engine::turn::{end_turn, start_game};

/// Backstop against an actual infinite loop bug -- a real game with these
/// decks should finish well under this (~53 turns per player before either
/// side decks out, if it hasn't ended on lore first).
const MAX_TURNS: u32 = 300;

fn find_name(cards: &[CardInstance], id: &str) -> String {
    cards
        .iter()
        .find(|c| c.instance_id == id)
        .map(|c| c.card.name.clone())
        .unwrap_or_else(|| "?".to_string())
}

fn describe_move(state: &GameState, mv: &Move) -> String {
    let active = &state.players[state.active_player];
    match mv {
        Move::Ink { instance_id } => format!("inks {}", find_name(&active.hand, instance_id)),
        Move::PlayCharacter {
            instance_id,
            enter_exerted,
        } => {
            let name = find_name(&active.hand, instance_id);
            if *enter_exerted {
                format!("plays {name} (entering exerted)")
            } else {
                format!("plays {name}")
            }
        }
        Move::Quest { instance_id } => format!("{} quests", find_name(&active.play, instance_id)),
        Move::Challenge {
            attacker_id,
            defender_id,
        } => {
            let opponent = &state.players[opponent_index(state.active_player)];
            format!(
                "{} challenges {}",
                find_name(&active.play, attacker_id),
                find_name(&opponent.play, defender_id)
            )
        }
        Move::Pass => "passes".to_string(),
    }
}

fn player_label(index: usize) -> &'static str {
    if index == 0 {
        "Player 1"
    } else {
        "Player 2"
    }
}

fn main() {
    let amber = load_deck("amber-vanilla-test.json");
    let steel = load_deck("steel-vanilla-test.json");

    let mut rng = system_rng();
    let mut state = create_game(amber, steel, &mut rng);
    start_game(&mut state);

    println!("=== Full game: Player 1 (amber-vanilla-test) vs Player 2 (steel-vanilla-test) ===\n");

    let mut turn_count: u32 = 0;

    loop {
        check_lore_victory(&mut state);
        if state.game_over.is_some() {
            break;
        }

        let active = &state.players[state.active_player];
        let hand_names: Vec<&str> = active.hand.iter().map(|c| c.card.name.as_str()).collect();
        println!(
            "--- Turn {}: {}'s turn (hand: {}) ---",
            state.turn_number,
            player_label(state.active_player),
            hand_names.join(", ")
        );

        loop {
            let mv = choose_move(&state);
            let description = describe_move(&state, &mv);
            let is_pass = matches!(mv, Move::Pass);

            apply_move(&mut state, &mv).expect("bot only ever chooses legal moves");
            if !is_pass {
                println!(
                    "[Turn {}] {}: {description} (lore {}-{})",
                    state.turn_number,
                    player_label(state.active_player),
                    state.players[0].lore,
                    state.players[1].lore
                );
            }

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
            println!("\nNo winner after {MAX_TURNS} turns -- stopping (safety valve).");
            break;
        }
    }

    println!();
    if let Some(over) = state.game_over {
        println!(
            "=== {} wins by {:?} on turn {} ===",
            player_label(over.winner),
            over.reason,
            state.turn_number
        );
    }
    println!(
        "Final lore -- Player 1: {}, Player 2: {}",
        state.players[0].lore, state.players[1].lore
    );
}
