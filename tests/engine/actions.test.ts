import { beforeEach, describe, expect, it } from "vitest";
import { clearHandlers } from "../../src/engine/events.ts";
import {
  IllegalActionError,
  challenge,
  inkCard,
  legalChallengeTargets,
  playCharacter,
  quest,
} from "../../src/engine/actions.ts";
import { createGame } from "../../src/engine/state.ts";
import { makeCard, makeInstance } from "./fixtures.ts";

beforeEach(() => {
  clearHandlers();
});

function newGame() {
  const deckA = Array.from({ length: 60 }, () => makeCard());
  const deckB = Array.from({ length: 60 }, () => makeCard());
  return createGame(deckA, deckB);
}

describe("inkCard", () => {
  it("moves an inkable card from hand to inkwell and marks ink used this turn", () => {
    const inkable = makeInstance({ inkwell: true, name: "Inkable" });
    const state = newGame();
    state.players[0].hand.push(inkable);

    inkCard(state, inkable.instanceId);

    expect(state.players[0].inkwell).toContainEqual(inkable);
    expect(state.players[0].hand).not.toContainEqual(inkable);
    expect(state.players[0].inkedThisTurn).toBe(true);
  });

  it("throws if the card cannot be inked", () => {
    const state = newGame();
    const notInkable = makeInstance({ inkwell: false });
    state.players[0].hand.push(notInkable);
    expect(() => inkCard(state, notInkable.instanceId)).toThrow(
      IllegalActionError,
    );
  });

  it("throws if a card has already been inked this turn", () => {
    const state = newGame();
    const a = makeInstance({ inkwell: true });
    const b = makeInstance({ inkwell: true });
    state.players[0].hand.push(a, b);
    inkCard(state, a.instanceId);
    expect(() => inkCard(state, b.instanceId)).toThrow(IllegalActionError);
  });
});

describe("playCharacter", () => {
  it("pays the ink cost and moves the character into play", () => {
    const state = newGame();
    const player = state.players[0];
    // put 3 ready ink in the inkwell
    for (let i = 0; i < 3; i++) player.inkwell.push(makeInstance());

    const character = makeInstance({ cost: 3, name: "Playable" });
    player.hand.push(character);

    playCharacter(state, character.instanceId);

    expect(player.play).toContainEqual(character);
    expect(character.playedThisTurn).toBe(true);
    expect(player.inkwell.every((i) => i.exerted)).toBe(true);
  });

  it("throws if there isn't enough available ink", () => {
    const state = newGame();
    const player = state.players[0];
    player.inkwell.push(makeInstance());

    const character = makeInstance({ cost: 3 });
    player.hand.push(character);

    expect(() => playCharacter(state, character.instanceId)).toThrow(
      IllegalActionError,
    );
  });

  it("lets a Bodyguard character enter play exerted", () => {
    const state = newGame();
    const player = state.players[0];
    const bodyguard = makeInstance({ cost: 0, keywords: ["Bodyguard"] });
    player.hand.push(bodyguard);

    playCharacter(state, bodyguard.instanceId, { enterExerted: true });

    expect(bodyguard.exerted).toBe(true);
  });

  it("throws if enterExerted is requested without Bodyguard", () => {
    const state = newGame();
    const player = state.players[0];
    const character = makeInstance({ cost: 0 });
    player.hand.push(character);

    expect(() =>
      playCharacter(state, character.instanceId, { enterExerted: true }),
    ).toThrow(IllegalActionError);
  });
});

describe("quest", () => {
  it("gains lore equal to the character's lore value and exerts it", () => {
    const state = newGame();
    const player = state.players[0];
    const character = makeInstance({ loreValue: 2 });
    player.play.push(character);

    quest(state, character.instanceId);

    expect(player.lore).toBe(2);
    expect(character.exerted).toBe(true);
  });

  it("throws if the character is already exerted", () => {
    const state = newGame();
    const player = state.players[0];
    const character = makeInstance();
    character.exerted = true;
    player.play.push(character);

    expect(() => quest(state, character.instanceId)).toThrow(
      IllegalActionError,
    );
  });

  it("throws if the character was just played this turn, even with Rush", () => {
    const state = newGame();
    const player = state.players[0];
    const character = makeInstance({ keywords: ["Rush"] });
    character.playedThisTurn = true;
    player.play.push(character);

    expect(() => quest(state, character.instanceId)).toThrow(
      IllegalActionError,
    );
  });
});

describe("legalChallengeTargets / challenge", () => {
  it("only allows targeting exerted opposing characters", () => {
    const state = newGame();
    const [, defenderPlayer] = state.players;
    const ready = makeInstance();
    const exerted = makeInstance();
    exerted.exerted = true;
    defenderPlayer.play.push(ready, exerted);

    const targets = legalChallengeTargets(state, 0);
    expect(targets).toEqual([exerted]);
  });

  it("forces targeting an exerted Bodyguard character if one exists", () => {
    const state = newGame();
    const [, defenderPlayer] = state.players;
    const exertedNormal = makeInstance();
    exertedNormal.exerted = true;
    const exertedBodyguard = makeInstance({ keywords: ["Bodyguard"] });
    exertedBodyguard.exerted = true;
    defenderPlayer.play.push(exertedNormal, exertedBodyguard);

    const targets = legalChallengeTargets(state, 0);
    expect(targets).toEqual([exertedBodyguard]);
  });

  it("applies Challenger +N to the attacker's damage and Resist +N to reduce damage taken", () => {
    const state = newGame();
    const [attackerPlayer, defenderPlayer] = state.players;

    const attacker = makeInstance({
      strength: 2,
      willpower: 3,
      keywords: ["Challenger"],
      text: "Challenger +2 (While challenging, this character gets +2 {S}.)",
    });
    const defender = makeInstance({
      strength: 3,
      willpower: 5,
      keywords: ["Resist"],
      text: "Resist +1 (Damage dealt to this character is reduced by 1.)",
    });
    defender.exerted = true;
    attackerPlayer.play.push(attacker);
    defenderPlayer.play.push(defender);

    challenge(state, attacker.instanceId, defender.instanceId);

    // attacker: 2 strength + 2 challenger = 4, minus defender's Resist +1 = 3 damage
    expect(defender.damage).toBe(3);
    // defender: 3 strength, minus attacker's 0 resist = 3 damage
    expect(attacker.damage).toBe(3);
    expect(attacker.exerted).toBe(true);
  });

  it("banishes a character once damage reaches its willpower", () => {
    const state = newGame();
    const [attackerPlayer, defenderPlayer] = state.players;

    const attacker = makeInstance({ strength: 5, willpower: 5 });
    const defender = makeInstance({ strength: 1, willpower: 4 });
    defender.exerted = true;
    attackerPlayer.play.push(attacker);
    defenderPlayer.play.push(defender);

    challenge(state, attacker.instanceId, defender.instanceId);

    expect(defenderPlayer.play).not.toContainEqual(defender);
    expect(defenderPlayer.discard).toContainEqual(defender);
    expect(attackerPlayer.play).toContainEqual(attacker);
  });

  it("throws if the defender is not exerted", () => {
    const state = newGame();
    const [attackerPlayer, defenderPlayer] = state.players;
    const attacker = makeInstance();
    const defender = makeInstance();
    attackerPlayer.play.push(attacker);
    defenderPlayer.play.push(defender);

    expect(() =>
      challenge(state, attacker.instanceId, defender.instanceId),
    ).toThrow(IllegalActionError);
  });
});
