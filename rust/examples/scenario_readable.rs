//! Same fixed scenario as examples/scenario.rs, but narrated as a readable
//! game log instead of a JSON dump. Run: cargo run --example scenario_readable
//!
//! Note for the TS comparison: in scenario.ts you can keep using `bagheera`,
//! `princeEric`, etc. freely after pushing them into `p1.play` -- same
//! object, shared reference. Here, once an instance is moved into a Vec
//! (`p1.play.push(bagheera)`), the local variable is gone. So the display
//! values we want to narrate (name, strength, willpower...) are cloned out
//! *before* the move, rather than read back off the original variable
//! afterward -- a very concrete case of the ownership rules we discussed.

use lorcana_sim::cards::find_card;
use lorcana_sim::engine::actions::{challenge, play_character, quest, PlayCharacterOptions};
use lorcana_sim::engine::state::{create_instance, GameState, Phase, PlayerState};
use lorcana_sim::engine::turn::end_turn;

fn empty_player(id: &str) -> PlayerState {
    PlayerState {
        id: id.to_string(),
        deck: Vec::new(),
        hand: Vec::new(),
        play: Vec::new(),
        inkwell: Vec::new(),
        discard: Vec::new(),
        lore: 0,
        inked_this_turn: false,
    }
}

fn main() {
    let bagheera = create_instance(find_card("Bagheera", "Cautious Explorer"));
    let prince_eric = create_instance(find_card("Prince Eric", "Noble Swordsman"));
    let beast = create_instance(find_card("Beast", "Thick-Skinned"));
    let mut tezuka = create_instance(find_card("Inspector Tezuka", "Resolute Officer"));
    tezuka.exerted = true;

    // Pulled out now, since the instances themselves get moved below.
    let bagheera_name = bagheera.card.name.clone();
    let bagheera_lore = bagheera.card.lore_value.unwrap_or(0);
    let beast_name = beast.card.name.clone();
    let beast_cost = beast.card.cost;
    let prince_eric_name = prince_eric.card.name.clone();
    let prince_eric_strength = prince_eric.card.strength.unwrap_or(0);
    let prince_eric_willpower = prince_eric.card.willpower.unwrap_or(0);
    let tezuka_name = tezuka.card.name.clone();
    let tezuka_strength = tezuka.card.strength.unwrap_or(0);
    let tezuka_willpower = tezuka.card.willpower.unwrap_or(0);

    let bagheera_id = bagheera.instance_id.clone();
    let prince_eric_id = prince_eric.instance_id.clone();
    let beast_id = beast.instance_id.clone();
    let tezuka_id = tezuka.instance_id.clone();

    let mut p1 = empty_player("player-1");
    p1.play.push(bagheera);
    p1.play.push(prince_eric);
    p1.hand.push(beast);
    for _ in 0..3 {
        p1.inkwell
            .push(create_instance(find_card("Bagheera", "Cautious Explorer")));
    }

    let mut p2 = empty_player("player-2");
    p2.play.push(tezuka);
    for _ in 0..5 {
        p2.deck
            .push(create_instance(find_card("Bagheera", "Cautious Explorer")));
    }

    let mut state = GameState {
        players: [p1, p2],
        turn_number: 1,
        active_player: 0,
        phase: Phase::Main,
    };

    println!("=== Lorcana Sim scenario ===");
    println!("Player 1's turn {} (main phase)\n", state.turn_number);

    quest(&mut state, &bagheera_id).unwrap();
    println!(
        "{bagheera_name} quests: gains {bagheera_lore} lore (Player 1 lore now {})",
        state.players[0].lore
    );

    play_character(&mut state, &beast_id, PlayCharacterOptions::default()).unwrap();
    println!(
        "{beast_name} enters play (cost {beast_cost}, paid with {beast_cost} ink) \
         - can't quest or challenge this turn (just played)"
    );

    println!("{prince_eric_name} challenges {tezuka_name}!");
    challenge(&mut state, &prince_eric_id, &tezuka_id).unwrap();

    let tezuka_banished = state.players[1]
        .discard
        .iter()
        .any(|c| c.instance_id == tezuka_id);
    let tezuka_damage = if tezuka_banished {
        state.players[1]
            .discard
            .iter()
            .find(|c| c.instance_id == tezuka_id)
            .unwrap()
            .damage
    } else {
        state.players[1]
            .play
            .iter()
            .find(|c| c.instance_id == tezuka_id)
            .unwrap()
            .damage
    };
    let prince_eric_damage = state.players[0]
        .play
        .iter()
        .find(|c| c.instance_id == prince_eric_id)
        .unwrap()
        .damage;

    println!(
        "  {prince_eric_name}: {prince_eric_strength} strength vs {tezuka_name}: \
         {tezuka_willpower} willpower -> {tezuka_damage} damage{}",
        if tezuka_banished { ", LETHAL" } else { "" }
    );
    println!(
        "  {tezuka_name}: {tezuka_strength} strength vs {prince_eric_name} -> {prince_eric_damage} damage"
    );
    if tezuka_banished {
        println!("  {tezuka_name} is banished!");
    }
    println!(
        "  {prince_eric_name} survives with {prince_eric_damage} damage ({prince_eric_willpower} willpower)\n"
    );

    let p2_hand_before = state.players[1].hand.len();
    end_turn(&mut state);
    println!("End of turn {}. Player 2's turn begins.", state.turn_number);
    println!(
        "Player 2 draws a card (hand size: {p2_hand_before} -> {})\n",
        state.players[1].hand.len()
    );

    println!("Final state:");
    println!("  Player 1 - lore: {}", state.players[0].lore);
    for c in &state.players[0].play {
        println!(
            "    {}: {}, {} damage{}",
            c.card.name,
            if c.exerted { "exerted" } else { "ready" },
            c.damage,
            if c.played_this_turn {
                " (played this turn)"
            } else {
                ""
            }
        );
    }
    println!(
        "  Player 2 - lore: {}, hand: {} card(s)",
        state.players[1].lore,
        state.players[1].hand.len()
    );
    if !state.players[1].discard.is_empty() {
        let names: Vec<&str> = state.players[1]
            .discard
            .iter()
            .map(|c| c.card.name.as_str())
            .collect();
        println!("    Discard: {}", names.join(", "));
    }
}
