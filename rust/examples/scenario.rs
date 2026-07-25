//! Fixed, non-random scenario run identically by the Rust and TS engines
//! (see parity/scenario.ts) to verify they agree bit-for-bit on real game
//! logic. No shuffling is involved -- the point is to check the ported
//! rules, not two independent PRNGs.
//!
//! Run: cargo run --example scenario

use lorcana_sim::cards::find_card;
use lorcana_sim::engine::actions::{challenge, play_character, quest, PlayCharacterOptions};
use lorcana_sim::engine::state::{create_instance, GameState, PlayerState, Phase};
use lorcana_sim::engine::turn::end_turn;
use serde_json::json;

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
    // Simulates having exerted on a prior turn (via Bodyguard entry or
    // questing), so it's a legal challenge target now.
    tezuka.exerted = true;

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
        game_over: None,
    };

    quest(&mut state, &bagheera_id).unwrap();
    play_character(&mut state, &beast_id, PlayCharacterOptions::default()).unwrap();
    challenge(&mut state, &prince_eric_id, &tezuka_id).unwrap();
    end_turn(&mut state);

    let mut p1_play: Vec<_> = state.players[0]
        .play
        .iter()
        .map(|c| {
            json!({
                "name": c.card.name,
                "exerted": c.exerted,
                "damage": c.damage,
                "playedThisTurn": c.played_this_turn,
            })
        })
        .collect();
    p1_play.sort_by_key(|v| v["name"].as_str().unwrap().to_string());

    let mut discard: Vec<String> = state.players[1]
        .discard
        .iter()
        .map(|c| c.card.name.clone())
        .collect();
    discard.sort();

    let phase = match state.phase {
        Phase::Ready => "ready",
        Phase::Set => "set",
        Phase::Draw => "draw",
        Phase::Main => "main",
    };

    let summary = json!({
        "turnNumber": state.turn_number,
        "activePlayer": state.active_player,
        "phase": phase,
        "player1": {
            "lore": state.players[0].lore,
            "play": p1_play,
        },
        "player2": {
            "lore": state.players[1].lore,
            "handSize": state.players[1].hand.len(),
            "discard": discard,
        },
    });

    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}
