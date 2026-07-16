use super::events::{emit, Event};
use super::state::{opponent_index, CardInstance, GameState, PlayerState};
use crate::cards::CardType;
use std::fmt;

#[derive(Debug)]
pub struct IllegalActionError(pub String);

impl fmt::Display for IllegalActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for IllegalActionError {}

fn err(msg: impl Into<String>) -> IllegalActionError {
    IllegalActionError(msg.into())
}

fn available_ink(player: &PlayerState) -> usize {
    player.inkwell.iter().filter(|i| !i.exerted).count()
}

fn pay_ink(player: &mut PlayerState, amount: usize) {
    let mut paid = 0;
    for ink in player.inkwell.iter_mut() {
        if paid >= amount {
            break;
        }
        if !ink.exerted {
            ink.exerted = true;
            paid += 1;
        }
    }
}

fn has_keyword(instance: &CardInstance, keyword: &str) -> bool {
    instance.card.keywords.iter().any(|k| k == keyword)
}

/// Generic keywords like "Challenger +2" or "Resist +1" carry their value in
/// the rules text rather than a separate field, so we read it out of there.
fn keyword_value(instance: &CardInstance, keyword: &str) -> i32 {
    let text = &instance.card.text;
    let Some(pos) = text.find(keyword) else {
        return 0;
    };
    let rest = text[pos + keyword.len()..].trim_start();
    let Some(rest) = rest.strip_prefix('+') else {
        return 0;
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

/// Quest always requires the character to be past its "wet ink" turn — Rush
/// does not lift this restriction, it only applies to challenging.
fn can_quest(instance: &CardInstance) -> bool {
    !instance.exerted && !instance.played_this_turn
}

/// Rush lets a character challenge the same turn it's played.
fn can_challenge_as_attacker(instance: &CardInstance) -> bool {
    if instance.exerted {
        return false;
    }
    if instance.played_this_turn && !has_keyword(instance, "Rush") {
        return false;
    }
    true
}

pub fn ink_card(state: &mut GameState, instance_id: &str) -> Result<(), IllegalActionError> {
    let player = &mut state.players[state.active_player];
    if player.inked_this_turn {
        return Err(err("Already inked a card this turn"));
    }
    let idx = player
        .hand
        .iter()
        .position(|c| c.instance_id == instance_id)
        .ok_or_else(|| err("Card not in hand"))?;
    if !player.hand[idx].card.inkwell {
        return Err(err(format!("{} cannot be inked", player.hand[idx].card.name)));
    }

    let instance = player.hand.remove(idx);
    let player_id = player.id.clone();
    let instance_id_owned = instance.instance_id.clone();
    player.inkwell.push(instance);
    player.inked_this_turn = true;

    emit(&Event::Ink {
        player_id,
        instance_id: instance_id_owned,
    });
    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PlayCharacterOptions {
    /// Bodyguard characters may choose to enter play already exerted.
    pub enter_exerted: bool,
}

pub fn play_character(
    state: &mut GameState,
    instance_id: &str,
    options: PlayCharacterOptions,
) -> Result<(), IllegalActionError> {
    let player = &mut state.players[state.active_player];
    let idx = player
        .hand
        .iter()
        .position(|c| c.instance_id == instance_id)
        .ok_or_else(|| err("Card not in hand"))?;

    {
        let instance = &player.hand[idx];
        if !instance.card.card_type.contains(&CardType::Character) {
            return Err(err(format!("{} is not a character", instance.card.name)));
        }
        if options.enter_exerted && !has_keyword(instance, "Bodyguard") {
            return Err(err(format!(
                "{} cannot enter play exerted",
                instance.card.name
            )));
        }
    }

    let cost = player.hand[idx].card.cost.max(0) as usize;
    if available_ink(player) < cost {
        return Err(err("Not enough available ink"));
    }
    pay_ink(player, cost);

    let mut instance = player.hand.remove(idx);
    instance.played_this_turn = true;
    instance.exerted = options.enter_exerted;
    let player_id = player.id.clone();
    let instance_id_owned = instance.instance_id.clone();
    player.play.push(instance);

    emit(&Event::Play {
        player_id,
        instance_id: instance_id_owned,
    });
    Ok(())
}

pub fn quest(state: &mut GameState, instance_id: &str) -> Result<(), IllegalActionError> {
    let player = &mut state.players[state.active_player];
    let idx = player
        .play
        .iter()
        .position(|c| c.instance_id == instance_id)
        .ok_or_else(|| err("Character not in play"))?;
    if !can_quest(&player.play[idx]) {
        return Err(err(format!("{} cannot quest", player.play[idx].card.name)));
    }

    player.play[idx].exerted = true;
    let lore_gained = player.play[idx].card.lore_value.unwrap_or(0);
    player.lore += lore_gained;
    let player_id = player.id.clone();
    let instance_id_owned = player.play[idx].instance_id.clone();

    emit(&Event::Quest {
        player_id,
        instance_id: instance_id_owned,
        lore_gained,
    });
    Ok(())
}

/// Opposing characters legal to challenge: must be exerted, and a Bodyguard
/// character must be chosen if the opponent has any exerted Bodyguard characters.
/// Returns instance ids rather than references, since Rust can't cheaply hand
/// back borrowed `&CardInstance`s tied to `state`'s lifetime here without
/// complicating every call site.
pub fn legal_challenge_targets(state: &GameState, attacker_index: usize) -> Vec<String> {
    let opponent = &state.players[opponent_index(attacker_index)];
    let exerted: Vec<&CardInstance> = opponent.play.iter().filter(|c| c.exerted).collect();
    let bodyguards: Vec<&CardInstance> = exerted
        .iter()
        .copied()
        .filter(|c| has_keyword(c, "Bodyguard"))
        .collect();
    let targets = if !bodyguards.is_empty() {
        bodyguards
    } else {
        exerted
    };
    targets.iter().map(|c| c.instance_id.clone()).collect()
}

fn banish_if_destroyed(player: &mut PlayerState, idx: usize) {
    let willpower = player.play[idx].card.willpower.unwrap_or(0);
    if player.play[idx].damage >= willpower {
        let instance = player.play.remove(idx);
        let owner_id = player.id.clone();
        let instance_id = instance.instance_id.clone();
        player.discard.push(instance);
        emit(&Event::Banish {
            owner_id,
            instance_id,
        });
    }
}

pub fn challenge(
    state: &mut GameState,
    attacker_id: &str,
    defender_id: &str,
) -> Result<(), IllegalActionError> {
    let attacker_index = state.active_player;
    let defender_index = opponent_index(attacker_index);

    {
        let attacker_player = &state.players[attacker_index];
        let attacker = attacker_player
            .play
            .iter()
            .find(|c| c.instance_id == attacker_id)
            .ok_or_else(|| err("Attacker not in play"))?;
        if !can_challenge_as_attacker(attacker) {
            return Err(err(format!("{} cannot challenge", attacker.card.name)));
        }
        let defender_player = &state.players[defender_index];
        if !defender_player
            .play
            .iter()
            .any(|c| c.instance_id == defender_id)
        {
            return Err(err("Defender not in play"));
        }
    }

    let legal_ids = legal_challenge_targets(state, attacker_index);
    if !legal_ids.iter().any(|id| id == defender_id) {
        return Err(err(
            "Must challenge an exerted character; a Bodyguard character must be \
             chosen if the opponent has one exerted",
        ));
    }

    let [p0, p1] = &mut state.players;
    let (attacker_player, defender_player) = if attacker_index == 0 {
        (p0, p1)
    } else {
        (p1, p0)
    };

    let attacker_idx = attacker_player
        .play
        .iter()
        .position(|c| c.instance_id == attacker_id)
        .unwrap();
    let defender_idx = defender_player
        .play
        .iter()
        .position(|c| c.instance_id == defender_id)
        .unwrap();

    attacker_player.play[attacker_idx].exerted = true;

    let attacker_strength = attacker_player.play[attacker_idx].card.strength.unwrap_or(0)
        + keyword_value(&attacker_player.play[attacker_idx], "Challenger");
    let defender_strength = defender_player.play[defender_idx].card.strength.unwrap_or(0);
    let attacker_resist = keyword_value(&attacker_player.play[attacker_idx], "Resist");
    let defender_resist = keyword_value(&defender_player.play[defender_idx], "Resist");

    let damage_to_defender = (attacker_strength - defender_resist).max(0);
    let damage_to_attacker = (defender_strength - attacker_resist).max(0);

    defender_player.play[defender_idx].damage += damage_to_defender;
    attacker_player.play[attacker_idx].damage += damage_to_attacker;

    emit(&Event::Challenge {
        attacker_id: attacker_id.to_string(),
        defender_id: defender_id.to_string(),
        damage_to_defender,
        damage_to_attacker,
    });

    banish_if_destroyed(attacker_player, attacker_idx);
    banish_if_destroyed(defender_player, defender_idx);

    Ok(())
}
