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
    fn inks_the_cheapest_inkable_card_first() {
        let mut state = new_game();
        let cheap = CardBuilder::new().inkwell(true).cost(1).build_instance();
        let expensive = CardBuilder::new().inkwell(true).cost(4).build_instance();
        let cheap_id = cheap.instance_id.clone();
        state.players[0].hand.push(expensive);
        state.players[0].hand.push(cheap);

        assert_eq!(choose_move(&state), Move::Ink { instance_id: cheap_id });
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
    fn takes_a_challenge_that_kills_the_defender_without_losing_the_attacker() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;

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
    fn does_not_take_a_challenge_that_would_lose_the_attacker() {
        let mut state = new_game();
        state.players[0].inked_this_turn = true;

        // Attacker would banish the defender, but also dies in the trade --
        // the crude bot is written to only take challenges it clearly wins.
        let attacker = CardBuilder::new().strength(5).willpower(3).build_instance();
        let attacker_id = attacker.instance_id.clone();
        state.players[0].play.push(attacker);

        let mut defender = CardBuilder::new().strength(5).willpower(4).build_instance();
        defender.exerted = true;
        state.players[1].play.push(defender);

        // Nothing else to do, so it should quest with the attacker instead of trading.
        assert_eq!(
            choose_move(&state),
            Move::Quest {
                instance_id: attacker_id
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
