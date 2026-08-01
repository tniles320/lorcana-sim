//! Plays many games between the two vanilla-heavy test decks and reports
//! win rate and average game length -- the minimal version of phase 6,
//! built specifically to let strategy changes be measured rather than
//! judged from a handful of individually-watched games.
//!
//! Who goes first is decided by a 2d6 roll each game (real tabletop
//! convention, re-rolling ties) -- going first is a real advantage, so
//! wins are tracked by deck name rather than by player slot, plus
//! separately by "went first" vs. "went second" to see whether that alone
//! is skewing results.
//!
//! Run: cargo run --example batch [game count, default 100]

use lorcana_sim::cards::load_deck;
use lorcana_sim::engine::state::{roll_for_first, system_rng, GameOverReason};
use lorcana_sim::sim::play_game;

fn main() {
    let games: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let amber = load_deck("amber-vanilla-test.json");
    let steel = load_deck("steel-vanilla-test.json");

    let mut amber_wins = 0u32;
    let mut steel_wins = 0u32;
    let mut first_player_wins = 0u32;
    let mut no_winner = 0u32;
    let mut deck_outs = 0u32;
    let mut total_turns: u64 = 0;
    let mut total_lore_amber: i64 = 0;
    let mut total_lore_steel: i64 = 0;

    for _ in 0..games {
        let mut rng = system_rng();
        let amber_goes_first = roll_for_first(&mut rng);
        let (deck_a, deck_b) = if amber_goes_first {
            (amber.clone(), steel.clone())
        } else {
            (steel.clone(), amber.clone())
        };

        let result = play_game(deck_a, deck_b, &mut rng);

        total_turns += result.turn_number as u64;
        let (amber_lore, steel_lore) = if amber_goes_first {
            (result.final_lore[0], result.final_lore[1])
        } else {
            (result.final_lore[1], result.final_lore[0])
        };
        total_lore_amber += amber_lore as i64;
        total_lore_steel += steel_lore as i64;

        match result.game_over {
            Some(over) => {
                let amber_won = (over.winner == 0) == amber_goes_first;
                if amber_won {
                    amber_wins += 1;
                } else {
                    steel_wins += 1;
                }
                if over.winner == 0 {
                    first_player_wins += 1;
                }
                if over.reason == GameOverReason::DeckOut {
                    deck_outs += 1;
                }
            }
            None => no_winner += 1,
        }
    }

    let pct = |n: u32| 100.0 * n as f64 / games as f64;

    println!("=== {games} games: amber-vanilla-test vs steel-vanilla-test (first player coin-flipped) ===");
    println!("amber-vanilla-test wins: {amber_wins} ({:.1}%)", pct(amber_wins));
    println!("steel-vanilla-test wins: {steel_wins} ({:.1}%)", pct(steel_wins));
    println!(
        "Went-first player wins: {first_player_wins} ({:.1}%)",
        pct(first_player_wins)
    );
    if no_winner > 0 {
        println!("No winner (safety valve hit): {no_winner} ({:.1}%)", pct(no_winner));
    }
    println!("Games ending in deck-out: {deck_outs} ({:.1}%)", pct(deck_outs));
    println!(
        "Average game length: {:.1} turns",
        total_turns as f64 / games as f64
    );
    println!(
        "Average final lore: amber-vanilla-test {:.1}, steel-vanilla-test {:.1}",
        total_lore_amber as f64 / games as f64,
        total_lore_steel as f64 / games as f64
    );
}
