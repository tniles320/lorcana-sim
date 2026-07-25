//! A deliberately crude heuristic bot (phase 5 of the scope doc). Ink and
//! play are simple first-class steps (they don't compete with anything --
//! ink/hand cards are a different resource than a character's own action).
//! Quest vs. challenge is the real either/or tension (a character can only
//! do one per turn), so those are scored on one shared scale and whichever
//! single action scores best across the whole board is taken. No archetype
//! awareness yet -- that comes later once real (non-vanilla) target decks
//! give us actual strategic identities to build around.

use crate::cards::CardType;
use crate::engine::actions::{
    can_challenge_as_attacker, can_quest, keyword_value, legal_challenge_targets, Move,
};
use crate::engine::state::{opponent_index, CardInstance, GameState};

/// How valuable/dangerous a character is -- used both to size up an
/// opposing threat worth attacking and to weigh the cost of losing one of
/// our own characters in a trade. Lore generation counts for the most
/// since it's the actual win condition; strength and willpower each count
/// as a flat "this is a problem on the battlefield" weight.
fn threat_value(card: &CardInstance) -> i32 {
    let lore = card.card.lore_value.unwrap_or(0);
    let strength = card.card.strength.unwrap_or(0);
    let willpower = card.card.willpower.unwrap_or(0);
    lore * 3 + strength + willpower
}

/// Net value of a specific challenge: the threat removed (full credit for
/// a kill, partial credit proportional to the chip damage otherwise) minus
/// the cost of losing the attacker, if this exchange would kill it too.
/// Combat in Lorcana is fully deterministic given stats, so "does the
/// attacker die" is a plain fact here, not a probability to weigh.
fn challenge_score(attacker: &CardInstance, defender: &CardInstance) -> i32 {
    let attacker_strength =
        attacker.card.strength.unwrap_or(0) + keyword_value(attacker, "Challenger");
    let defender_resist = keyword_value(defender, "Resist");
    let damage_to_defender = (attacker_strength - defender_resist).max(0);
    let defender_willpower = defender.card.willpower.unwrap_or(0).max(1);
    let kills_defender = defender.damage + damage_to_defender >= defender_willpower;

    let defender_strength = defender.card.strength.unwrap_or(0);
    let attacker_resist = keyword_value(attacker, "Resist");
    let damage_to_attacker = (defender_strength - attacker_resist).max(0);
    let attacker_dies =
        attacker.damage + damage_to_attacker >= attacker.card.willpower.unwrap_or(0).max(1);

    let value_removed = if kills_defender {
        threat_value(defender)
    } else {
        threat_value(defender) * damage_to_defender / defender_willpower
    };
    let cost = if attacker_dies { threat_value(attacker) } else { 0 };

    value_removed - cost
}

/// Picks a single move for the active player. Call in a loop, applying each
/// move via `actions::apply_move`, until it returns `Move::Pass`.
pub fn choose_move(state: &GameState) -> Move {
    let active = &state.players[state.active_player];
    let available_ink = active.inkwell.iter().filter(|i| !i.exerted).count() as i32;
    let currently_playable = |c: &CardInstance| {
        c.card.card_type.contains(&CardType::Character) && c.card.cost <= available_ink
    };

    // 1. Ink the cheapest inkable card in hand that we couldn't already
    //    afford to play -- inking a card we could play right now would
    //    throw away a real play just to gain ink we may not even need yet.
    if !active.inked_this_turn
        && let Some(card) = active
            .hand
            .iter()
            .filter(|c| c.card.inkwell && !currently_playable(c))
            .min_by_key(|c| c.card.cost)
    {
        return Move::Ink {
            instance_id: card.instance_id.clone(),
        };
    }

    // 2. Play the biggest character we can currently afford.
    if let Some(card) = active
        .hand
        .iter()
        .filter(|c| currently_playable(c))
        .max_by_key(|c| c.card.cost)
    {
        return Move::PlayCharacter {
            instance_id: card.instance_id.clone(),
            enter_exerted: false,
        };
    }

    // 3. For every character that can quest or challenge, score both and
    //    take whichever single action scores best across the whole board.
    let opponent = &state.players[opponent_index(state.active_player)];
    let mut best: Option<(i32, Move)> = None;
    let mut consider = |score: i32, mv: Move| {
        if best.as_ref().is_none_or(|(best_score, _)| score > *best_score) {
            best = Some((score, mv));
        }
    };

    for character in &active.play {
        if can_quest(character) {
            consider(
                character.card.lore_value.unwrap_or(0),
                Move::Quest {
                    instance_id: character.instance_id.clone(),
                },
            );
        }
        if can_challenge_as_attacker(character) {
            for target_id in legal_challenge_targets(state, state.active_player) {
                let defender = opponent
                    .play
                    .iter()
                    .find(|c| c.instance_id == target_id)
                    .expect("legal_challenge_targets only returns ids present in opponent.play");
                consider(
                    challenge_score(character, defender),
                    Move::Challenge {
                        attacker_id: character.instance_id.clone(),
                        defender_id: target_id,
                    },
                );
            }
        }
    }

    match best {
        Some((score, mv)) if score > 0 => mv,
        _ => Move::Pass,
    }
}
