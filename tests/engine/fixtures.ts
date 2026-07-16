import type { Card } from "../../src/cards/types.ts";
import type { CardInstance } from "../../src/engine/state.ts";

let counter = 0;

/** A static card definition, for building deck arrays (Card[]) passed to createGame. */
export function makeCard(overrides: Partial<Card> = {}): Card {
  counter += 1;
  return {
    id: `test_${counter}`,
    name: overrides.name ?? `Test Character ${counter}`,
    version: overrides.version ?? "Test Version",
    type: overrides.type ?? ["Character"],
    ink: overrides.ink ?? "Amber",
    cost: overrides.cost ?? 1,
    inkwell: overrides.inkwell ?? true,
    classifications: overrides.classifications ?? [],
    text: overrides.text ?? "",
    keywords: overrides.keywords ?? [],
    strength: overrides.strength ?? 1,
    willpower: overrides.willpower ?? 1,
    loreValue: overrides.loreValue ?? 1,
    moveCost: overrides.moveCost ?? null,
    rarity: overrides.rarity ?? "Common",
    setCode: overrides.setCode ?? "1",
  };
}

let instanceCounter = 0;

/** A live in-game copy, for pushing directly into a player's hand/play/inkwell/discard. */
export function makeInstance(overrides: Partial<Card> = {}): CardInstance {
  instanceCounter += 1;
  return {
    instanceId: `test_inst_${instanceCounter}`,
    card: makeCard(overrides),
    damage: 0,
    exerted: false,
    playedThisTurn: false,
  };
}
