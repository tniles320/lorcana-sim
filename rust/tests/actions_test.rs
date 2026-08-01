mod common;

use common::CardBuilder;
use lorcana_sim::cards::Card;
use lorcana_sim::engine::actions::{
    challenge, ink_card, legal_challenge_targets, play_character, quest, PlayCharacterOptions,
};
use lorcana_sim::engine::events::clear_handlers;
use lorcana_sim::engine::state::{create_game, system_rng, GameState};

fn deck_of(size: usize) -> Vec<Card> {
    (0..size).map(|_| CardBuilder::new().build()).collect()
}

fn new_game() -> GameState {
    clear_handlers();
    let mut rng = system_rng();
    create_game(deck_of(60), deck_of(60), &mut rng)
}

mod ink_card_tests {
    use super::*;

    #[test]
    fn moves_an_inkable_card_from_hand_to_inkwell_and_marks_ink_used_this_turn() {
        let mut state = new_game();
        let inkable = CardBuilder::new().inkwell(true).name("Inkable").build_instance();
        let id = inkable.instance_id.clone();
        state.players[0].hand.push(inkable);

        ink_card(&mut state, &id).unwrap();

        assert!(state.players[0].inkwell.iter().any(|c| c.instance_id == id));
        assert!(!state.players[0].hand.iter().any(|c| c.instance_id == id));
        assert!(state.players[0].inked_this_turn);
    }

    #[test]
    fn throws_if_the_card_cannot_be_inked() {
        let mut state = new_game();
        let not_inkable = CardBuilder::new().inkwell(false).build_instance();
        let id = not_inkable.instance_id.clone();
        state.players[0].hand.push(not_inkable);

        assert!(ink_card(&mut state, &id).is_err());
    }

    #[test]
    fn throws_if_a_card_has_already_been_inked_this_turn() {
        let mut state = new_game();
        let a = CardBuilder::new().inkwell(true).build_instance();
        let b = CardBuilder::new().inkwell(true).build_instance();
        let a_id = a.instance_id.clone();
        let b_id = b.instance_id.clone();
        state.players[0].hand.push(a);
        state.players[0].hand.push(b);

        ink_card(&mut state, &a_id).unwrap();
        assert!(ink_card(&mut state, &b_id).is_err());
    }
}

mod play_character_tests {
    use super::*;

    #[test]
    fn pays_the_ink_cost_and_moves_the_character_into_play() {
        let mut state = new_game();
        for _ in 0..3 {
            state.players[0].inkwell.push(CardBuilder::new().build_instance());
        }
        let character = CardBuilder::new().cost(3).name("Playable").build_instance();
        let id = character.instance_id.clone();
        state.players[0].hand.push(character);

        play_character(&mut state, &id, PlayCharacterOptions::default()).unwrap();

        let played = state.players[0].play.iter().find(|c| c.instance_id == id);
        assert!(played.is_some());
        assert!(played.unwrap().played_this_turn);
        assert!(state.players[0].inkwell.iter().all(|i| i.exerted));
    }

    #[test]
    fn throws_if_there_isnt_enough_available_ink() {
        let mut state = new_game();
        state.players[0].inkwell.push(CardBuilder::new().build_instance());
        let character = CardBuilder::new().cost(3).build_instance();
        let id = character.instance_id.clone();
        state.players[0].hand.push(character);

        assert!(play_character(&mut state, &id, PlayCharacterOptions::default()).is_err());
    }

    #[test]
    fn lets_a_bodyguard_character_enter_play_exerted() {
        let mut state = new_game();
        let bodyguard = CardBuilder::new()
            .cost(0)
            .keywords(&["Bodyguard"])
            .build_instance();
        let id = bodyguard.instance_id.clone();
        state.players[0].hand.push(bodyguard);

        play_character(
            &mut state,
            &id,
            PlayCharacterOptions { enter_exerted: true },
        )
        .unwrap();

        let played = state.players[0].play.iter().find(|c| c.instance_id == id).unwrap();
        assert!(played.exerted);
    }

    #[test]
    fn throws_if_enter_exerted_is_requested_without_bodyguard() {
        let mut state = new_game();
        let character = CardBuilder::new().cost(0).build_instance();
        let id = character.instance_id.clone();
        state.players[0].hand.push(character);

        let result = play_character(&mut state, &id, PlayCharacterOptions { enter_exerted: true });
        assert!(result.is_err());
    }
}

mod quest_tests {
    use super::*;

    #[test]
    fn gains_lore_equal_to_the_characters_lore_value_and_exerts_it() {
        let mut state = new_game();
        let character = CardBuilder::new().lore_value(2).build_instance();
        let id = character.instance_id.clone();
        state.players[0].play.push(character);

        quest(&mut state, &id).unwrap();

        assert_eq!(state.players[0].lore, 2);
        let questor = state.players[0].play.iter().find(|c| c.instance_id == id).unwrap();
        assert!(questor.exerted);
    }

    #[test]
    fn throws_if_the_character_is_already_exerted() {
        let mut state = new_game();
        let mut character = CardBuilder::new().build_instance();
        character.exerted = true;
        let id = character.instance_id.clone();
        state.players[0].play.push(character);

        assert!(quest(&mut state, &id).is_err());
    }

    #[test]
    fn throws_if_the_character_was_just_played_this_turn_even_with_rush() {
        let mut state = new_game();
        let mut character = CardBuilder::new().keywords(&["Rush"]).build_instance();
        character.played_this_turn = true;
        let id = character.instance_id.clone();
        state.players[0].play.push(character);

        assert!(quest(&mut state, &id).is_err());
    }
}

mod challenge_tests {
    use super::*;

    #[test]
    fn only_allows_targeting_exerted_opposing_characters() {
        let mut state = new_game();
        let ready = CardBuilder::new().build_instance();
        let mut exerted = CardBuilder::new().build_instance();
        exerted.exerted = true;
        let exerted_id = exerted.instance_id.clone();
        state.players[1].play.push(ready);
        state.players[1].play.push(exerted);

        let attacker = CardBuilder::new().build_instance();
        let targets = legal_challenge_targets(&state, 0, &attacker);
        assert_eq!(targets, vec![exerted_id]);
    }

    #[test]
    fn forces_targeting_an_exerted_bodyguard_character_if_one_exists() {
        let mut state = new_game();
        let mut exerted_normal = CardBuilder::new().build_instance();
        exerted_normal.exerted = true;
        let mut exerted_bodyguard = CardBuilder::new().keywords(&["Bodyguard"]).build_instance();
        exerted_bodyguard.exerted = true;
        let bodyguard_id = exerted_bodyguard.instance_id.clone();
        state.players[1].play.push(exerted_normal);
        state.players[1].play.push(exerted_bodyguard);

        let attacker = CardBuilder::new().build_instance();
        let targets = legal_challenge_targets(&state, 0, &attacker);
        assert_eq!(targets, vec![bodyguard_id]);
    }

    #[test]
    fn evasive_defender_cannot_be_targeted_by_a_non_evasive_attacker() {
        let mut state = new_game();
        let mut evasive_defender = CardBuilder::new().keywords(&["Evasive"]).build_instance();
        evasive_defender.exerted = true;
        state.players[1].play.push(evasive_defender);

        let attacker = CardBuilder::new().build_instance();
        let targets = legal_challenge_targets(&state, 0, &attacker);
        assert!(targets.is_empty());
    }

    #[test]
    fn evasive_defender_can_be_targeted_by_an_evasive_attacker() {
        let mut state = new_game();
        let mut evasive_defender = CardBuilder::new().keywords(&["Evasive"]).build_instance();
        evasive_defender.exerted = true;
        let defender_id = evasive_defender.instance_id.clone();
        state.players[1].play.push(evasive_defender);

        let attacker = CardBuilder::new().keywords(&["Evasive"]).build_instance();
        let targets = legal_challenge_targets(&state, 0, &attacker);
        assert_eq!(targets, vec![defender_id]);
    }

    #[test]
    fn bodyguard_forcing_is_skipped_if_the_only_bodyguard_is_unreachable_via_evasive() {
        let mut state = new_game();
        let mut evasive_bodyguard = CardBuilder::new()
            .keywords(&["Evasive", "Bodyguard"])
            .build_instance();
        evasive_bodyguard.exerted = true;
        let mut plain_exerted = CardBuilder::new().build_instance();
        plain_exerted.exerted = true;
        let plain_exerted_id = plain_exerted.instance_id.clone();
        state.players[1].play.push(evasive_bodyguard);
        state.players[1].play.push(plain_exerted);

        // The attacker can't reach the Evasive Bodyguard at all, so it's
        // not "able" to choose it -- the plain exerted character is the
        // only legal target.
        let attacker = CardBuilder::new().build_instance();
        let targets = legal_challenge_targets(&state, 0, &attacker);
        assert_eq!(targets, vec![plain_exerted_id]);
    }

    #[test]
    fn applies_challenger_bonus_and_resist_reduction() {
        let mut state = new_game();
        let attacker = CardBuilder::new()
            .strength(2)
            .willpower(10) // comfortably survives the counter-damage below
            .keywords(&["Challenger"])
            .text("Challenger +2 (While challenging, this character gets +2 {S}.)")
            .build_instance();
        let mut defender = CardBuilder::new()
            .strength(3)
            .willpower(5)
            .keywords(&["Resist"])
            .text("Resist +1 (Damage dealt to this character is reduced by 1.)")
            .build_instance();
        defender.exerted = true;

        let attacker_id = attacker.instance_id.clone();
        let defender_id = defender.instance_id.clone();
        state.players[0].play.push(attacker);
        state.players[1].play.push(defender);

        challenge(&mut state, &attacker_id, &defender_id).unwrap();

        // attacker: 2 strength + 2 challenger = 4, minus defender's Resist +1 = 3 damage
        let defender_after = state.players[1]
            .play
            .iter()
            .find(|c| c.instance_id == defender_id)
            .unwrap();
        assert_eq!(defender_after.damage, 3);

        // defender: 3 strength, minus attacker's 0 resist = 3 damage
        let attacker_after = state.players[0]
            .play
            .iter()
            .find(|c| c.instance_id == attacker_id)
            .unwrap();
        assert_eq!(attacker_after.damage, 3);
        assert!(attacker_after.exerted);
    }

    #[test]
    fn banishes_a_character_once_damage_reaches_its_willpower() {
        let mut state = new_game();
        let attacker = CardBuilder::new().strength(5).willpower(5).build_instance();
        let mut defender = CardBuilder::new().strength(1).willpower(4).build_instance();
        defender.exerted = true;

        let attacker_id = attacker.instance_id.clone();
        let defender_id = defender.instance_id.clone();
        state.players[0].play.push(attacker);
        state.players[1].play.push(defender);

        challenge(&mut state, &attacker_id, &defender_id).unwrap();

        assert!(!state.players[1].play.iter().any(|c| c.instance_id == defender_id));
        assert!(state.players[1].discard.iter().any(|c| c.instance_id == defender_id));
        assert!(state.players[0].play.iter().any(|c| c.instance_id == attacker_id));
    }

    #[test]
    fn throws_if_the_defender_is_not_exerted() {
        let mut state = new_game();
        let attacker = CardBuilder::new().build_instance();
        let defender = CardBuilder::new().build_instance();
        let attacker_id = attacker.instance_id.clone();
        let defender_id = defender.instance_id.clone();
        state.players[0].play.push(attacker);
        state.players[1].play.push(defender);

        assert!(challenge(&mut state, &attacker_id, &defender_id).is_err());
    }
}
