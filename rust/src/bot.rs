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
use crate::engine::state::{opponent_index, CardInstance, GameState, PlayerState, LORE_TO_WIN};

/// Within this many lore points of winning, stop worrying about exposing
/// characters to retaliation and just push for the win -- a first guess,
/// not a derived number.
const NEAR_WIN_MARGIN: i32 = 3;

/// Above this cost, a card generally isn't worth keeping in an opening
/// hand -- we want plays in the first 3-4 turns. Deck/strategy-specific
/// exceptions (a control shell wanting a big finisher) aren't modeled yet.
const MULLIGAN_MAX_KEEPABLE_COST: i32 = 4;

/// More than this many uninkable cards in an opening hand is a real risk:
/// an uninkable card that doesn't get played can never become ink either,
/// so it's just dead until drawn into naturally.
const MAX_UNINKABLE_IN_OPENING_HAND: usize = 1;

/// Decides which opening-hand cards to mulligan (put back and redraw).
/// Cost-based for now: put back anything above the keepable curve, and if
/// more than one remaining card is uninkable, keep only the cheapest of
/// those and put back the rest even though their cost was otherwise fine.
pub fn decide_mulligan(hand: &[CardInstance]) -> Vec<String> {
    let mut to_mulligan: Vec<String> = hand
        .iter()
        .filter(|c| c.card.cost > MULLIGAN_MAX_KEEPABLE_COST)
        .map(|c| c.instance_id.clone())
        .collect();

    let mut remaining_uninkable: Vec<&CardInstance> = hand
        .iter()
        .filter(|c| !to_mulligan.contains(&c.instance_id))
        .filter(|c| !c.card.inkwell)
        .collect();

    if remaining_uninkable.len() > MAX_UNINKABLE_IN_OPENING_HAND {
        remaining_uninkable.sort_by_key(|c| c.card.cost);
        for c in remaining_uninkable
            .into_iter()
            .skip(MAX_UNINKABLE_IN_OPENING_HAND)
        {
            to_mulligan.push(c.instance_id.clone());
        }
    }

    to_mulligan
}

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

/// How many other copies of this exact card (same name and version) are
/// also sitting in hand -- extra copies beyond the first are increasingly
/// expendable as ink.
fn duplicate_count(hand: &[CardInstance], card: &CardInstance) -> i32 {
    hand.iter()
        .filter(|c| {
            c.instance_id != card.instance_id
                && c.card.name == card.card.name
                && c.card.version == card.card.version
        })
        .count() as i32
}

/// How good an ink candidate this card is: the further its cost is beyond
/// what we can already afford, the more "dead" it is in hand right now
/// (worth converting rather than waiting many turns to play it); each
/// duplicate copy still in hand adds to that, since only so many copies of
/// the same card are useful to hold onto at once. This one formula covers
/// both "ink the expensive card I can't play for a while" (small inkwell,
/// big gap) and "ink the redundant extra copy" (large inkwell, gaps shrink
/// toward zero for everything, so duplicates become the deciding factor).
fn ink_score(card: &CardInstance, available_ink: i32, hand: &[CardInstance]) -> i32 {
    let gap = (card.card.cost - available_ink).max(0);
    let duplicates = duplicate_count(hand, card);
    gap + duplicates * 2
}

struct ChallengeOutcome {
    /// Threat removed (full credit for a kill, partial credit proportional
    /// to the chip damage otherwise) minus the cost of losing the attacker,
    /// if this exchange kills it too.
    net_score: i32,
    attacker_dies: bool,
    /// The attacker's damage value after this fight resolves -- relevant
    /// even when it survives, since that's the exposure it carries into
    /// the opponent's next turn.
    attacker_damage_after: i32,
}

/// Evaluates a specific challenge. Combat in Lorcana is fully deterministic
/// given stats, so "does the attacker die" is a plain fact here, not a
/// probability to weigh.
fn evaluate_challenge(attacker: &CardInstance, defender: &CardInstance) -> ChallengeOutcome {
    let attacker_strength =
        attacker.card.strength.unwrap_or(0) + keyword_value(attacker, "Challenger");
    let defender_resist = keyword_value(defender, "Resist");
    let damage_to_defender = (attacker_strength - defender_resist).max(0);
    let defender_willpower = defender.card.willpower.unwrap_or(0).max(1);
    let kills_defender = defender.damage + damage_to_defender >= defender_willpower;

    let defender_strength = defender.card.strength.unwrap_or(0);
    let attacker_resist = keyword_value(attacker, "Resist");
    let damage_to_attacker = (defender_strength - attacker_resist).max(0);
    let attacker_damage_after = attacker.damage + damage_to_attacker;
    let attacker_dies = attacker_damage_after >= attacker.card.willpower.unwrap_or(0).max(1);

    let value_removed = if kills_defender {
        threat_value(defender)
    } else {
        threat_value(defender) * damage_to_defender / defender_willpower
    };
    let cost = if attacker_dies { threat_value(attacker) } else { 0 };

    ChallengeOutcome {
        net_score: value_removed - cost,
        attacker_dies,
        attacker_damage_after,
    }
}

/// How many distinct opposing characters would have a free (attacker-
/// survives) kill against a character of ours with these stats, once it's
/// exposed (exerted) on their turn. Checks every opposing character
/// regardless of its current exerted/wet-ink state, since all of them will
/// be readied and past wet ink by the time the opponent's own next turn
/// starts. Each opposing character can only make one challenge per turn,
/// which is exactly why this is a *count* rather than a yes/no -- only
/// that many of our exposed characters can actually be punished, no matter
/// how many we expose. Only considers single attackers, not combinations
/// of smaller ones ganging up, and only the opponent's current board, not
/// hypothetical future plays we can't see coming (their hand is hidden).
fn punisher_count(my_strength: i32, my_willpower_remaining: i32, my_resist: i32, opponent: &PlayerState) -> i32 {
    opponent
        .play
        .iter()
        .filter(|enemy| {
            let enemy_strength =
                enemy.card.strength.unwrap_or(0) + keyword_value(enemy, "Challenger");
            let enemy_resist = keyword_value(enemy, "Resist");
            let damage_to_me = (enemy_strength - my_resist).max(0);
            let they_kill_me = damage_to_me >= my_willpower_remaining;

            let damage_to_them = (my_strength - enemy_resist).max(0);
            let enemy_willpower = enemy.card.willpower.unwrap_or(0).max(1);
            let i_kill_them_back = enemy.damage + damage_to_them >= enemy_willpower;

            they_kill_me && !i_kill_them_back
        })
        .count() as i32
}

/// The expected cost of leaving `character` exposed (exerted) given its
/// willpower remaining after whatever action we're scoring, and
/// `total_potential_exposure`: how many of our characters are already
/// exerted from earlier actions this turn, *plus* how many more (including
/// `character` itself) could still act this turn and might also end up
/// exposed. This has to be forward-looking, not just "how many are
/// already exposed" -- if every character is evaluated independently
/// assuming nothing else has been risked yet, nothing ever volunteers to
/// go first, and the board freezes solid even when we vastly outnumber
/// the opponent's actual punishing capacity.
///
/// A single opposing threat can only punish one exposed character per
/// turn, so once our total potential exposure exceeds the number of
/// distinct threats capable of punishing this specific character, no
/// individual character should be modeled as certain to be the one that
/// dies -- we don't know which one the opponent would pick, and most of
/// the pool survives regardless of their choice.
///
/// Also waived once `lore_after_action` is close enough to winning that
/// board safety no longer matters more than closing out the game.
fn retaliation_risk(
    character: &CardInstance,
    willpower_remaining: i32,
    total_potential_exposure: i32,
    opponent: &PlayerState,
    lore_after_action: i32,
) -> i32 {
    if lore_after_action >= LORE_TO_WIN - NEAR_WIN_MARGIN {
        return 0;
    }
    let my_strength = character.card.strength.unwrap_or(0);
    let my_resist = keyword_value(character, "Resist");
    let punishers = punisher_count(my_strength, willpower_remaining, my_resist, opponent);
    if punishers == 0 || total_potential_exposure > punishers {
        0
    } else {
        threat_value(character)
    }
}

/// Picks a single move for the active player. Call in a loop, applying each
/// move via `actions::apply_move`, until it returns `Move::Pass`.
pub fn choose_move(state: &GameState) -> Move {
    let active = &state.players[state.active_player];
    let available_ink = active.inkwell.iter().filter(|i| !i.exerted).count() as i32;
    let currently_playable = |c: &CardInstance| {
        c.card.card_type.contains(&CardType::Character) && c.card.cost <= available_ink
    };

    // 1. Ink the best-scoring inkable card in hand that we couldn't already
    //    afford to play -- inking a card we could play right now would
    //    throw away a real play just to gain ink we may not even need yet.
    if !active.inked_this_turn
        && let Some(card) = active
            .hand
            .iter()
            .filter(|c| c.card.inkwell && !currently_playable(c))
            .max_by_key(|c| ink_score(c, available_ink, &active.hand))
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

    // How many of our characters are already exerted from earlier actions
    // this turn, plus how many more (unexerted, but capable of quest or
    // challenge) could still act -- the full pool that might end up
    // exposed to the opponent's next turn, evaluated once up front so
    // every character's risk is judged against the whole picture rather
    // than each in isolation assuming it'd be the only one exposed.
    let already_exerted_count = active.play.iter().filter(|c| c.exerted).count() as i32;
    let remaining_actionable_count = active
        .play
        .iter()
        .filter(|c| !c.exerted && (can_quest(c) || can_challenge_as_attacker(c)))
        .count() as i32;
    let total_potential_exposure = already_exerted_count + remaining_actionable_count;

    for character in &active.play {
        if can_quest(character) {
            let lore_gained = character.card.lore_value.unwrap_or(0);
            let willpower_remaining =
                character.card.willpower.unwrap_or(0) - character.damage;
            let risk = retaliation_risk(
                character,
                willpower_remaining,
                total_potential_exposure,
                opponent,
                active.lore + lore_gained,
            );
            consider(
                lore_gained - risk,
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
                let outcome = evaluate_challenge(character, defender);
                let risk = if outcome.attacker_dies {
                    0 // already priced into net_score's cost term
                } else {
                    let willpower_remaining = character.card.willpower.unwrap_or(0)
                        - outcome.attacker_damage_after;
                    retaliation_risk(
                        character,
                        willpower_remaining,
                        total_potential_exposure,
                        opponent,
                        active.lore,
                    )
                };
                consider(
                    outcome.net_score - risk,
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
