import { describe, expect, it } from "vitest";
import { loadDeck } from "../../src/cards/load.ts";
import { createGame } from "../../src/engine/state.ts";
import { makeCard } from "./fixtures.ts";

describe("createGame", () => {
  it("shuffles each deck, draws a 7-card opening hand, and leaves the rest in the deck", () => {
    const deckA = Array.from({ length: 60 }, () => makeCard());
    const deckB = Array.from({ length: 60 }, () => makeCard());
    const state = createGame(deckA, deckB);

    for (const player of state.players) {
      expect(player.hand).toHaveLength(7);
      expect(player.deck).toHaveLength(53);
      expect(player.play).toHaveLength(0);
      expect(player.inkwell).toHaveLength(0);
      expect(player.discard).toHaveLength(0);
      expect(player.lore).toBe(0);
    }
  });

  it("is deterministic given a seeded rng", () => {
    const deckA = Array.from({ length: 10 }, (_, i) =>
      makeCard({ name: `Card ${i}` }),
    );
    const deckB = Array.from({ length: 10 }, (_, i) =>
      makeCard({ name: `Card ${i}` }),
    );

    const seeded = () => {
      let seed = 42;
      return () => {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        return seed / 2147483648;
      };
    };

    const stateA = createGame(deckA, deckB, { rng: seeded() });
    const stateB = createGame(deckA, deckB, { rng: seeded() });

    expect(stateA.players[0].hand.map((c) => c.card.name)).toEqual(
      stateB.players[0].hand.map((c) => c.card.name),
    );
  });

  it("loads the real test decks and builds a 60-card game", () => {
    const amber = loadDeck("amber-vanilla-test.json");
    const steel = loadDeck("steel-vanilla-test.json");
    expect(amber).toHaveLength(60);
    expect(steel).toHaveLength(60);

    const state = createGame(amber, steel);
    expect(state.players[0].hand).toHaveLength(7);
    expect(state.players[1].hand).toHaveLength(7);
  });
});
