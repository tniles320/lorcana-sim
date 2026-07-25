mod common;

use common::CardBuilder;
use lorcana_sim::bot::choose_move;
use lorcana_sim::cards::Card;
use lorcana_sim::engine::actions::{legal_moves, Move};
use lorcana_sim::engine::events::clear_handlers;
use lorcana_sim::engine::state::{create_game, system_rng, GameState};

fn deck_of(size: usize) -> Vec<Card> {
    (0..size).map(|_| CardBuilder::new().build()).collect()
}

/// Every test here inspects hand/play contents directly or relies on
/// `choose_move`'s search over them, so we clear the dealt opening hands --
/// unlike actions_test.rs, which always targets a specific instance_id and
/// so doesn't care what else is sitting in hand.
fn new_game() -> GameState {
    clear_handlers();
    let mut rng = system_rng();
    let mut state = create_game(deck_of(60), deck_of(60), &mut rng);
    state.players[0].hand.clear();
    state.players[1].hand.clear();
    state
}

mod legal_moves_tests {
    use super::*;

    #[test]
    fn always_includes_pass() {
        let state = new_game();
        assert!(legal_moves(&state).contains(&Move::Pass));
    }

    #[test]
    fn offers_ink_for_each_inkable_hand_card_when_not_yet_inked() {
        let mut state = new_game();
        let inkable = CardBuilder::new().inkwell(true).build_instance();
        let id = inkable.instance_id.clone();
        state.players[0].hand.push(inkable);

        let moves = legal_moves(&state);
        assert!(moves.contains(&Move::Ink { instance_id: id }));
    }

    #[test]
    fn offers_no_ink_moves_once_already_inked_this_turn() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;
        state.players[0]
            .hand
            .push(CardBuilder::new().inkwell(true).build_instance());

        let moves = legal_moves(&state);
        assert!(!moves.iter().any(|m| matches!(m, Move::Ink { .. })));
    }

    #[test]
    fn offers_play_character_only_for_affordable_cards() {
        let mut state = new_game();
        for _ in 0..2 {
            state.players[0].inkwell.push(CardBuilder::new().build_instance());
        }
        let affordable = CardBuilder::new().cost(2).build_instance();
        let too_expensive = CardBuilder::new().cost(3).build_instance();
        let affordable_id = affordable.instance_id.clone();
        let too_expensive_id = too_expensive.instance_id.clone();
        state.players[0].hand.push(affordable);
        state.players[0].hand.push(too_expensive);

        let moves = legal_moves(&state);
        assert!(moves.contains(&Move::PlayCharacter {
            instance_id: affordable_id,
            enter_exerted: false
        }));
        assert!(!moves.iter().any(|m| matches!(
            m,
            Move::PlayCharacter { instance_id, .. } if *instance_id == too_expensive_id
        )));
    }

    #[test]
    fn offers_an_enter_exerted_option_for_bodyguard_characters() {
        let mut state = new_game();
        let bodyguard = CardBuilder::new().cost(0).keywords(&["Bodyguard"]).build_instance();
        let id = bodyguard.instance_id.clone();
        state.players[0].hand.push(bodyguard);

        let moves = legal_moves(&state);
        assert!(moves.contains(&Move::PlayCharacter {
            instance_id: id.clone(),
            enter_exerted: false
        }));
        assert!(moves.contains(&Move::PlayCharacter {
            instance_id: id,
            enter_exerted: true
        }));
    }

    #[test]
    fn offers_quest_only_for_characters_that_can_quest() {
        let mut state = new_game();
        let ready = CardBuilder::new().build_instance();
        let mut exerted = CardBuilder::new().build_instance();
        exerted.exerted = true;
        let ready_id = ready.instance_id.clone();
        state.players[0].play.push(ready);
        state.players[0].play.push(exerted);

        let moves = legal_moves(&state);
        assert!(moves.contains(&Move::Quest {
            instance_id: ready_id
        }));
        assert_eq!(
            moves.iter().filter(|m| matches!(m, Move::Quest { .. })).count(),
            1
        );
    }
}

mod choose_move_tests {
    use super::*;

    #[test]
    fn inks_the_least_reachable_card_when_theres_no_duplicate_pressure() {
        let mut state = new_game();
        let cheap = CardBuilder::new().inkwell(true).cost(1).build_instance();
        let expensive = CardBuilder::new().inkwell(true).cost(4).build_instance();
        let expensive_id = expensive.instance_id.clone();
        state.players[0].hand.push(cheap);
        state.players[0].hand.push(expensive);

        // available_ink = 0, so the cost-4 card is much further from being
        // playable than the cost-1 card -- more "dead" in hand right now,
        // and thus the better one to convert to ink.
        assert_eq!(
            choose_move(&state),
            Move::Ink {
                instance_id: expensive_id
            }
        );
    }

    #[test]
    fn prefers_inking_a_duplicate_over_a_similarly_unreachable_unique_card() {
        let mut state = new_game();
        let original = CardBuilder::new().name("Bagheera").cost(3).build_instance();
        let duplicate = CardBuilder::new().name("Bagheera").cost(3).build_instance();
        let unique = CardBuilder::new().name("Someone Else").cost(3).build_instance();
        let original_id = original.instance_id.clone();
        let duplicate_id = duplicate.instance_id.clone();
        let unique_id = unique.instance_id.clone();
        state.players[0].hand.push(original);
        state.players[0].hand.push(duplicate);
        state.players[0].hand.push(unique);

        // All three cost the same (identical gap), but the two Bagheeras
        // are duplicates of each other, so inking one of them scores
        // higher than inking the unique card.
        match choose_move(&state) {
            Move::Ink { instance_id } => {
                assert!(instance_id == original_id || instance_id == duplicate_id);
                assert_ne!(instance_id, unique_id);
            }
            other => panic!("expected an Ink move, got {other:?}"),
        }
    }

    #[test]
    fn prefers_a_duplicate_over_a_slightly_less_reachable_unique_card() {
        let mut state = new_game();
        for _ in 0..3 {
            state.players[0]
                .inkwell
                .push(CardBuilder::new().build_instance());
        }
        // available_ink = 3
        let original = CardBuilder::new().name("Bagheera").cost(4).build_instance(); // gap 1, +2 dup = 3
        let duplicate = CardBuilder::new().name("Bagheera").cost(4).build_instance(); // gap 1, +2 dup = 3
        let unique = CardBuilder::new().name("Someone Else").cost(5).build_instance(); // gap 2
        let original_id = original.instance_id.clone();
        let duplicate_id = duplicate.instance_id.clone();
        let unique_id = unique.instance_id.clone();
        state.players[0].hand.push(original);
        state.players[0].hand.push(duplicate);
        state.players[0].hand.push(unique);

        // The unique card has a bigger raw gap (2 vs 1), but the duplicate
        // bonus outweighs that -- ties should go to thinning duplicates.
        match choose_move(&state) {
            Move::Ink { instance_id } => {
                assert!(instance_id == original_id || instance_id == duplicate_id);
                assert_ne!(instance_id, unique_id);
            }
            other => panic!("expected an Ink move, got {other:?}"),
        }
    }

    #[test]
    fn does_not_ink_a_card_it_could_currently_afford_to_play() {
        let mut state = new_game();
        state.players[0].inkwell.push(CardBuilder::new().build_instance()); // 1 ready ink
        let affordable = CardBuilder::new().cost(1).build_instance();
        let affordable_id = affordable.instance_id.clone();
        state.players[0].hand.push(affordable);

        // Even though it's the only (and thus "cheapest") inkable card,
        // inking it would throw away the only play available this turn.
        assert_eq!(
            choose_move(&state),
            Move::PlayCharacter {
                instance_id: affordable_id,
                enter_exerted: false
            }
        );
    }

    #[test]
    fn inks_a_card_it_could_not_currently_afford_to_play_instead() {
        let mut state = new_game();
        state.players[0].inkwell.push(CardBuilder::new().build_instance()); // 1 ready ink
        let affordable = CardBuilder::new().cost(1).build_instance();
        let unaffordable = CardBuilder::new().cost(5).build_instance();
        let unaffordable_id = unaffordable.instance_id.clone();
        state.players[0].hand.push(affordable);
        state.players[0].hand.push(unaffordable);

        // The cost-1 card stays in hand to be played; the cost-5 card can't
        // be played this turn regardless, so it's safe to convert to ink.
        assert_eq!(
            choose_move(&state),
            Move::Ink {
                instance_id: unaffordable_id
            }
        );
    }

    #[test]
    fn plays_the_most_expensive_affordable_character_once_inked() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;
        for _ in 0..3 {
            state.players[0].inkwell.push(CardBuilder::new().build_instance());
        }
        let small = CardBuilder::new().cost(1).build_instance();
        let big = CardBuilder::new().cost(3).build_instance();
        let big_id = big.instance_id.clone();
        state.players[0].hand.push(small);
        state.players[0].hand.push(big);

        assert_eq!(
            choose_move(&state),
            Move::PlayCharacter {
                instance_id: big_id,
                enter_exerted: false
            }
        );
    }

    #[test]
    fn takes_a_challenge_that_clearly_outscores_questing() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;

        // Free kill (attacker survives): challenge_score is the defender's
        // full threat_value, easily beating a 1-lore quest.
        let attacker = CardBuilder::new().strength(5).willpower(5).build_instance();
        let attacker_id = attacker.instance_id.clone();
        state.players[0].play.push(attacker);

        let mut defender = CardBuilder::new().strength(1).willpower(4).build_instance();
        defender.exerted = true;
        let defender_id = defender.instance_id.clone();
        state.players[1].play.push(defender);

        assert_eq!(
            choose_move(&state),
            Move::Challenge {
                attacker_id,
                defender_id
            }
        );
    }

    #[test]
    fn avoids_a_clearly_bad_trade_in_favor_of_questing() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;

        // Attacker (lore 3, so questing scores 3) would banish the defender
        // but also dies -- and the defender (a vanilla 5/4 with only 1 lore)
        // isn't worth nearly as much as the attacker, so the trade nets
        // clearly negative. Questing should win easily.
        let attacker = CardBuilder::new()
            .strength(5)
            .willpower(3)
            .lore_value(3)
            .build_instance();
        let attacker_id = attacker.instance_id.clone();
        state.players[0].play.push(attacker);

        let mut defender = CardBuilder::new().strength(5).willpower(4).build_instance();
        defender.exerted = true;
        state.players[1].play.push(defender);

        assert_eq!(
            choose_move(&state),
            Move::Quest {
                instance_id: attacker_id
            }
        );
    }

    #[test]
    fn takes_a_worthwhile_trade_even_though_the_attacker_dies() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;

        // Attacker (low lore, low value) trades into a much higher-value
        // defender (lore 3 -- a real lore-engine threat). Net score should
        // still favor the trade even though the attacker doesn't survive.
        let attacker = CardBuilder::new()
            .strength(4)
            .willpower(1)
            .lore_value(1)
            .build_instance();
        let attacker_id = attacker.instance_id.clone();
        state.players[0].play.push(attacker);

        let mut defender = CardBuilder::new()
            .strength(5)
            .willpower(3)
            .lore_value(3)
            .build_instance();
        defender.exerted = true;
        let defender_id = defender.instance_id.clone();
        state.players[1].play.push(defender);

        assert_eq!(
            choose_move(&state),
            Move::Challenge {
                attacker_id,
                defender_id
            }
        );
    }

    #[test]
    fn chips_a_high_threat_target_even_without_a_kill() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;

        // Attacker can't kill this turn (str 1 vs. willpower 6), and takes
        // no real risk doing it (defender's strength is low), but the
        // defender is a big lore threat -- partial chip-damage credit
        // should still outscore a 1-lore quest.
        let attacker = CardBuilder::new().strength(1).willpower(5).build_instance();
        let attacker_id = attacker.instance_id.clone();
        state.players[0].play.push(attacker);

        let mut defender = CardBuilder::new()
            .strength(1)
            .willpower(6)
            .lore_value(3)
            .build_instance();
        defender.exerted = true;
        let defender_id = defender.instance_id.clone();
        state.players[1].play.push(defender);

        assert_eq!(
            choose_move(&state),
            Move::Challenge {
                attacker_id,
                defender_id
            }
        );
    }

    #[test]
    fn quests_when_theres_nothing_better_to_do() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;
        let quester = CardBuilder::new().build_instance();
        let quester_id = quester.instance_id.clone();
        state.players[0].play.push(quester);

        assert_eq!(
            choose_move(&state),
            Move::Quest {
                instance_id: quester_id
            }
        );
    }

    #[test]
    fn passes_when_there_is_nothing_legal_to_do() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;
        assert_eq!(choose_move(&state), Move::Pass);
    }
}
