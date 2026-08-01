//! Plays one full game between the two vanilla-heavy test decks, using the
//! crude heuristic bot for both sides, and prints a readable move-by-move
//! log. This is the first time the engine runs to actual completion instead
//! of a hand-scripted scenario.
//!
//! Run: cargo run --example full_game

use lorcana_sim::bot::{choose_move, decide_mulligan};
use lorcana_sim::cards::load_deck;
use lorcana_sim::engine::actions::{apply_move, Move};
use lorcana_sim::engine::state::{
    check_lore_victory, create_game, mulligan, opponent_index, system_rng, CardInstance, GameState,
    PlayerState,
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

/// Damage dealt to a character by this specific hit (current damage minus
/// what it had going in) and whether it was banished -- checks `play` first
/// (survived) then `discard` (banished; the instance still holds its final
/// damage value there since `challenge()` just moves it, doesn't reset it).
fn damage_and_banish_status(
    play: &[CardInstance],
    discard: &[CardInstance],
    id: &str,
    damage_before: i32,
) -> (i32, bool) {
    if let Some(c) = play.iter().find(|c| c.instance_id == id) {
        (c.damage - damage_before, false)
    } else if let Some(c) = discard.iter().find(|c| c.instance_id == id) {
        (c.damage - damage_before, true)
    } else {
        (0, false)
    }
}

/// "Name (cost, strength/willpower, ready/drying/exerted, damage if any)" --
/// shows the stats the bot actually reasons about, so it's clear why a
/// given character was judged safe or risky to act with. "Drying" means
/// just played this turn -- still in play and unexerted, but (absent Rush)
/// can't quest or challenge until its owner's next Ready phase, so it's
/// meaningfully different from truly being available to act with.
fn describe_character(c: &CardInstance) -> String {
    let status = if c.exerted {
        "exerted"
    } else if c.played_this_turn {
        "drying"
    } else {
        "ready"
    };
    let dmg = if c.damage > 0 {
        format!(", {} dmg", c.damage)
    } else {
        String::new()
    };
    format!(
        "{} ({}c {}/{}, {status}{dmg})",
        c.card.name,
        c.card.cost,
        c.card.strength.unwrap_or(0),
        c.card.willpower.unwrap_or(0)
    )
}

fn describe_board(player: &PlayerState) -> String {
    if player.play.is_empty() {
        "(empty)".to_string()
    } else {
        player
            .play
            .iter()
            .map(describe_character)
            .collect::<Vec<_>>()
            .join(", ")
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

    // Coin flip for who goes first -- going first is a real advantage
    // (inking a turn earlier), so which deck lands in "Player 1" (the
    // slot that goes first and skips its opening draw) shouldn't be fixed.
    let amber_goes_first = rng() < 0.5;
    let (deck_a, deck_b, name_a, name_b) = if amber_goes_first {
        (amber, steel, "amber-vanilla-test", "steel-vanilla-test")
    } else {
        (steel, amber, "steel-vanilla-test", "amber-vanilla-test")
    };

    let mut state = create_game(deck_a, deck_b, &mut rng);

    println!("=== Full game: Player 1 ({name_a}) vs Player 2 ({name_b}) ===");
    println!("Player 1 goes first.\n");

    for (i, player) in state.players.iter_mut().enumerate() {
        let to_mulligan = decide_mulligan(&player.hand);
        if to_mulligan.is_empty() {
            println!("{}: keeps opening hand", player_label(i));
        } else {
            let names: Vec<String> = to_mulligan
                .iter()
                .map(|id| find_name(&player.hand, id))
                .collect();
            println!("{}: mulligans {}", player_label(i), names.join(", "));
            mulligan(player, &to_mulligan, &mut rng);
        }
    }
    println!();

    start_game(&mut state);

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

            // For challenges, snapshot pre-hit damage so we can report the
            // delta (and banish status) once the move resolves.
            let challenge_snapshot = if let Move::Challenge {
                attacker_id,
                defender_id,
            } = &mv
            {
                let active = &state.players[state.active_player];
                let opponent = &state.players[opponent_index(state.active_player)];
                let attacker_damage_before = active
                    .play
                    .iter()
                    .find(|c| &c.instance_id == attacker_id)
                    .map(|c| c.damage)
                    .unwrap_or(0);
                let defender_damage_before = opponent
                    .play
                    .iter()
                    .find(|c| &c.instance_id == defender_id)
                    .map(|c| c.damage)
                    .unwrap_or(0);
                Some((
                    attacker_id.clone(),
                    defender_id.clone(),
                    attacker_damage_before,
                    defender_damage_before,
                ))
            } else {
                None
            };

            apply_move(&mut state, &mv).expect("bot only ever chooses legal moves");

            let combat_detail = challenge_snapshot.map(
                |(attacker_id, defender_id, attacker_damage_before, defender_damage_before)| {
                    let active = &state.players[state.active_player];
                    let opponent = &state.players[opponent_index(state.active_player)];
                    let (defender_dmg, defender_banished) = damage_and_banish_status(
                        &opponent.play,
                        &opponent.discard,
                        &defender_id,
                        defender_damage_before,
                    );
                    let (attacker_dmg, attacker_banished) = damage_and_banish_status(
                        &active.play,
                        &active.discard,
                        &attacker_id,
                        attacker_damage_before,
                    );
                    format!(
                        " [deals {defender_dmg} dmg{}, takes {attacker_dmg} dmg{}]",
                        if defender_banished {
                            " -- BANISHED"
                        } else {
                            ""
                        },
                        if attacker_banished {
                            " -- BANISHED"
                        } else {
                            ""
                        }
                    )
                },
            );

            if !is_pass {
                println!(
                    "[Turn {}] {}: {description}{} (lore {}-{})",
                    state.turn_number,
                    player_label(state.active_player),
                    combat_detail.unwrap_or_default(),
                    state.players[0].lore,
                    state.players[1].lore
                );
            }

            check_lore_victory(&mut state);
            if is_pass || state.game_over.is_some() {
                break;
            }
        }

        println!(
            "  Board -- P1: {} | P2: {}\n",
            describe_board(&state.players[0]),
            describe_board(&state.players[1])
        );

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
