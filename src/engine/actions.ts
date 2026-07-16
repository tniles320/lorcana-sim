import { emit } from "./events.ts";
import {
  type CardInstance,
  type GameState,
  type PlayerState,
  activePlayer,
  opponentOf,
} from "./state.ts";

export class IllegalActionError extends Error {}

function availableInk(player: PlayerState): number {
  return player.inkwell.filter((i) => !i.exerted).length;
}

function payInk(player: PlayerState, amount: number): void {
  const available = player.inkwell.filter((i) => !i.exerted);
  for (let i = 0; i < amount; i++) {
    available[i].exerted = true;
  }
}

function hasKeyword(instance: CardInstance, keyword: string): boolean {
  return instance.card.keywords.includes(keyword);
}

/** Generic keywords like "Challenger +2" or "Resist +1" carry their value in the
 * rules text rather than a separate field, so we read it out of there. */
function keywordValue(instance: CardInstance, keyword: string): number {
  const match = instance.card.text.match(new RegExp(`${keyword} \\+(\\d+)`));
  return match ? Number(match[1]) : 0;
}

/** Quest always requires the character to be past its "wet ink" turn — Rush
 * does not lift this restriction, it only applies to challenging. */
function canQuest(instance: CardInstance): boolean {
  return !instance.exerted && !instance.playedThisTurn;
}

/** Rush lets a character challenge the same turn it's played. */
function canChallengeAsAttacker(instance: CardInstance): boolean {
  if (instance.exerted) return false;
  if (instance.playedThisTurn && !hasKeyword(instance, "Rush")) return false;
  return true;
}

export function inkCard(state: GameState, instanceId: string): void {
  const player = activePlayer(state);
  if (player.inkedThisTurn) {
    throw new IllegalActionError("Already inked a card this turn");
  }
  const idx = player.hand.findIndex((c) => c.instanceId === instanceId);
  if (idx === -1) throw new IllegalActionError("Card not in hand");
  const instance = player.hand[idx];
  if (!instance.card.inkwell) {
    throw new IllegalActionError(`${instance.card.name} cannot be inked`);
  }
  player.hand.splice(idx, 1);
  player.inkwell.push(instance);
  player.inkedThisTurn = true;
  emit("ink", { state, player, instance });
}

export interface PlayCharacterOptions {
  /** Bodyguard characters may choose to enter play already exerted. */
  enterExerted?: boolean;
}

export function playCharacter(
  state: GameState,
  instanceId: string,
  options: PlayCharacterOptions = {},
): void {
  const player = activePlayer(state);
  const idx = player.hand.findIndex((c) => c.instanceId === instanceId);
  if (idx === -1) throw new IllegalActionError("Card not in hand");
  const instance = player.hand[idx];
  if (!instance.card.type.includes("Character")) {
    throw new IllegalActionError(`${instance.card.name} is not a character`);
  }
  if (options.enterExerted && !hasKeyword(instance, "Bodyguard")) {
    throw new IllegalActionError(
      `${instance.card.name} cannot enter play exerted`,
    );
  }

  const cost = instance.card.cost;
  if (availableInk(player) < cost) {
    throw new IllegalActionError("Not enough available ink");
  }
  payInk(player, cost);

  player.hand.splice(idx, 1);
  instance.playedThisTurn = true;
  instance.exerted = Boolean(options.enterExerted);
  player.play.push(instance);
  emit("play", { state, player, instance });
}

export function quest(state: GameState, instanceId: string): void {
  const player = activePlayer(state);
  const instance = player.play.find((c) => c.instanceId === instanceId);
  if (!instance) throw new IllegalActionError("Character not in play");
  if (!canQuest(instance)) {
    throw new IllegalActionError(`${instance.card.name} cannot quest`);
  }
  instance.exerted = true;
  const loreGained = instance.card.loreValue ?? 0;
  player.lore += loreGained;
  emit("quest", { state, player, instance, loreGained });
}

/** Opposing characters legal to challenge: must be exerted, and a Bodyguard
 * character must be chosen if the opponent has any exerted Bodyguard characters. */
export function legalChallengeTargets(
  state: GameState,
  attackerIndex: 0 | 1,
): CardInstance[] {
  const opponent = opponentOf(state, attackerIndex);
  const exerted = opponent.play.filter((c) => c.exerted);
  const bodyguards = exerted.filter((c) => hasKeyword(c, "Bodyguard"));
  return bodyguards.length > 0 ? bodyguards : exerted;
}

function banishIfDestroyed(
  state: GameState,
  owner: PlayerState,
  instance: CardInstance,
): void {
  const willpower = instance.card.willpower ?? 0;
  if (instance.damage >= willpower) {
    owner.play = owner.play.filter((c) => c.instanceId !== instance.instanceId);
    owner.discard.push(instance);
    emit("banish", { state, owner, instance });
  }
}

export function challenge(
  state: GameState,
  attackerId: string,
  defenderId: string,
): void {
  const attackerIndex = state.activePlayer;
  const attackerPlayer = activePlayer(state);
  const defenderPlayer = opponentOf(state, attackerIndex);

  const attacker = attackerPlayer.play.find(
    (c) => c.instanceId === attackerId,
  );
  if (!attacker) throw new IllegalActionError("Attacker not in play");
  if (!canChallengeAsAttacker(attacker)) {
    throw new IllegalActionError(`${attacker.card.name} cannot challenge`);
  }

  const defender = defenderPlayer.play.find(
    (c) => c.instanceId === defenderId,
  );
  if (!defender) throw new IllegalActionError("Defender not in play");

  const legalTargets = legalChallengeTargets(state, attackerIndex);
  if (!legalTargets.some((c) => c.instanceId === defenderId)) {
    throw new IllegalActionError(
      "Must challenge an exerted character; a Bodyguard character must be " +
        "chosen if the opponent has one exerted",
    );
  }

  attacker.exerted = true;

  const attackerStrength =
    (attacker.card.strength ?? 0) + keywordValue(attacker, "Challenger");
  const defenderStrength = defender.card.strength ?? 0;
  const attackerResist = keywordValue(attacker, "Resist");
  const defenderResist = keywordValue(defender, "Resist");

  const damageToDefender = Math.max(0, attackerStrength - defenderResist);
  const damageToAttacker = Math.max(0, defenderStrength - attackerResist);

  defender.damage += damageToDefender;
  attacker.damage += damageToAttacker;

  emit("challenge", {
    state,
    attacker,
    defender,
    damageToDefender,
    damageToAttacker,
  });

  banishIfDestroyed(state, attackerPlayer, attacker);
  banishIfDestroyed(state, defenderPlayer, defender);
}
