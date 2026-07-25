//! A deliberately crude heuristic bot (phase 5 of the scope doc): ink if
//! possible, play the biggest affordable character, take clearly favorable
//! challenges, quest with anything else that can, otherwise pass. No
//! archetype awareness yet -- that comes later once real (non-vanilla)
//! target decks give us actual strategic identities to build around.

use crate::cards::CardType;
use crate::engine::actions::{
    can_challenge_as_attacker, can_quest, keyword_value, legal_challenge_targets, Move,
};
use crate::engine::state::{opponent_index, GameState};

/// Picks a single move for the active player. Call in a loop, applying each
/// move via `actions::apply_move`, until it returns `Move::Pass`.
pub fn choose_move(state: &GameState) -> Move {
    let active = &state.players[state.active_player];

    // 1. Ink the cheapest inkable card in hand (once per turn), keeping
    //    higher-cost cards available to actually play.
    if !active.inked_this_turn
        && let Some(card) = active
            .hand
            .iter()
            .filter(|c| c.card.inkwell)
            .min_by_key(|c| c.card.cost)
    {
        return Move::Ink {
            instance_id: card.instance_id.clone(),
        };
    }

    // 2. Play the biggest character we can currently afford.
    let available_ink = active.inkwell.iter().filter(|i| !i.exerted).count() as i32;
    if let Some(card) = active
        .hand
        .iter()
        .filter(|c| c.card.card_type.contains(&CardType::Character))
        .filter(|c| c.card.cost <= available_ink)
        .max_by_key(|c| c.card.cost)
    {
        return Move::PlayCharacter {
            instance_id: card.instance_id.clone(),
            enter_exerted: false,
        };
    }

    // 3. Take any challenge that banishes the defender without losing the attacker.
    let opponent = &state.players[opponent_index(state.active_player)];
    for attacker in active.play.iter().filter(|c| can_challenge_as_attacker(c)) {
        for target_id in legal_challenge_targets(state, state.active_player) {
            let defender = opponent
                .play
                .iter()
                .find(|c| c.instance_id == target_id)
                .expect("legal_challenge_targets only returns ids present in opponent.play");

            let attacker_strength =
                attacker.card.strength.unwrap_or(0) + keyword_value(attacker, "Challenger");
            let defender_resist = keyword_value(defender, "Resist");
            let damage_to_defender = (attacker_strength - defender_resist).max(0);
            let kills_defender =
                defender.damage + damage_to_defender >= defender.card.willpower.unwrap_or(0);

            let defender_strength = defender.card.strength.unwrap_or(0);
            let attacker_resist = keyword_value(attacker, "Resist");
            let damage_to_attacker = (defender_strength - attacker_resist).max(0);
            let attacker_survives =
                attacker.damage + damage_to_attacker < attacker.card.willpower.unwrap_or(0);

            if kills_defender && attacker_survives {
                return Move::Challenge {
                    attacker_id: attacker.instance_id.clone(),
                    defender_id: target_id,
                };
            }
        }
    }

    // 4. Quest with anything that can.
    if let Some(quester) = active.play.iter().find(|c| can_quest(c)) {
        return Move::Quest {
            instance_id: quester.instance_id.clone(),
        };
    }

    Move::Pass
}
