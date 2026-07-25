//! Plays many games between the two vanilla-heavy test decks and reports
//! win rate and average game length -- the minimal version of phase 6,
//! built specifically to let strategy changes be measured rather than
//! judged from a handful of individually-watched games.
//!
//! Run: cargo run --example batch [game count, default 100]

use lorcana_sim::cards::load_deck;
use lorcana_sim::engine::state::{system_rng, GameOverReason};
use lorcana_sim::sim::play_game;

fn main() {
    let games: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let amber = load_deck("amber-vanilla-test.json");
    let steel = load_deck("steel-vanilla-test.json");

    let mut player1_wins = 0u32;
    let mut player2_wins = 0u32;
    let mut no_winner = 0u32;
    let mut deck_outs = 0u32;
    let mut total_turns: u64 = 0;
    let mut total_lore = [0i64, 0i64];

    for _ in 0..games {
        let mut rng = system_rng();
        let result = play_game(amber.clone(), steel.clone(), &mut rng);

        total_turns += result.turn_number as u64;
        total_lore[0] += result.final_lore[0] as i64;
        total_lore[1] += result.final_lore[1] as i64;

        match result.game_over {
            Some(over) => {
                if over.winner == 0 {
                    player1_wins += 1;
                } else {
                    player2_wins += 1;
                }
                if over.reason == GameOverReason::DeckOut {
                    deck_outs += 1;
                }
            }
            None => no_winner += 1,
        }
    }

    let pct = |n: u32| 100.0 * n as f64 / games as f64;

    println!("=== {games} games: amber-vanilla-test (P1) vs steel-vanilla-test (P2) ===");
    println!("Player 1 wins: {player1_wins} ({:.1}%)", pct(player1_wins));
    println!("Player 2 wins: {player2_wins} ({:.1}%)", pct(player2_wins));
    if no_winner > 0 {
        println!("No winner (safety valve hit): {no_winner} ({:.1}%)", pct(no_winner));
    }
    println!("Games ending in deck-out: {deck_outs} ({:.1}%)", pct(deck_outs));
    println!(
        "Average game length: {:.1} turns",
        total_turns as f64 / games as f64
    );
    println!(
        "Average final lore: Player 1 {:.1}, Player 2 {:.1}",
        total_lore[0] as f64 / games as f64,
        total_lore[1] as f64 / games as f64
    );
}
