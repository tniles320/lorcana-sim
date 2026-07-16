import { beforeEach, describe, expect, it } from "vitest";
import { clearHandlers } from "../../src/engine/events.ts";
import { createGame } from "../../src/engine/state.ts";
import { endTurn, startGame } from "../../src/engine/turn.ts";
import { makeCard } from "./fixtures.ts";

beforeEach(() => {
  clearHandlers();
});

function newGame() {
  const deckA = Array.from({ length: 60 }, () => makeCard());
  const deckB = Array.from({ length: 60 }, () => makeCard());
  return createGame(deckA, deckB);
}

describe("startGame", () => {
  it("does not draw for the first player's first turn", () => {
    const state = newGame();
    startGame(state);
    expect(state.phase).toBe("main");
    expect(state.players[0].hand).toHaveLength(7);
  });
});

describe("endTurn", () => {
  it("draws for the second player's first turn", () => {
    const state = newGame();
    startGame(state);
    endTurn(state);
    expect(state.activePlayer).toBe(1);
    expect(state.players[1].hand).toHaveLength(8);
    expect(state.turnNumber).toBe(1);
  });

  it("increments turnNumber only when wrapping back to player 0", () => {
    const state = newGame();
    startGame(state);
    endTurn(state);
    expect(state.turnNumber).toBe(1);
    endTurn(state);
    expect(state.activePlayer).toBe(0);
    expect(state.turnNumber).toBe(2);
  });

  it("resets exerted, playedThisTurn, and inkedThisTurn only on that player's own Ready phase", () => {
    const state = newGame();
    startGame(state);

    const p1 = state.players[0];
    const character = p1.hand.pop()!;
    character.exerted = true;
    character.playedThisTurn = true;
    p1.play.push(character);

    const ink = p1.hand.pop()!;
    ink.exerted = true;
    p1.inkwell.push(ink);
    p1.inkedThisTurn = true;

    endTurn(state); // -> player 2's turn; player 1's flags untouched
    expect(character.exerted).toBe(true);
    expect(p1.inkedThisTurn).toBe(true);

    endTurn(state); // -> back to player 1's turn; Ready phase runs for player 1
    expect(character.exerted).toBe(false);
    expect(character.playedThisTurn).toBe(false);
    expect(ink.exerted).toBe(false);
    expect(p1.inkedThisTurn).toBe(false);
  });
});
