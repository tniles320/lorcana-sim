/**
 * Same fixed scenario as scenario.ts, but narrated as a readable game log
 * instead of a JSON dump. Run: npx tsx parity/scenario-readable.ts
 */
import { findCard } from "../src/cards/load.ts";
import { challenge, playCharacter, quest } from "../src/engine/actions.ts";
import {
  createInstance,
  type GameState,
  type PlayerState,
} from "../src/engine/state.ts";
import { endTurn } from "../src/engine/turn.ts";

function emptyPlayer(id: string): PlayerState {
  return {
    id,
    deck: [],
    hand: [],
    play: [],
    inkwell: [],
    discard: [],
    lore: 0,
    inkedThisTurn: false,
  };
}

const bagheera = createInstance(findCard("Bagheera", "Cautious Explorer"));
const princeEric = createInstance(findCard("Prince Eric", "Noble Swordsman"));
const beast = createInstance(findCard("Beast", "Thick-Skinned"));
const tezuka = createInstance(findCard("Inspector Tezuka", "Resolute Officer"));
tezuka.exerted = true;

const p1 = emptyPlayer("player-1");
p1.play.push(bagheera, princeEric);
p1.hand.push(beast);
for (let i = 0; i < 3; i++) {
  p1.inkwell.push(createInstance(findCard("Bagheera", "Cautious Explorer")));
}

const p2 = emptyPlayer("player-2");
p2.play.push(tezuka);
for (let i = 0; i < 5; i++) {
  p2.deck.push(createInstance(findCard("Bagheera", "Cautious Explorer")));
}

const state: GameState = {
  players: [p1, p2],
  turnNumber: 1,
  activePlayer: 0,
  phase: "main",
};

console.log("=== Lorcana Sim scenario ===");
console.log(`Player 1's turn ${state.turnNumber} (main phase)\n`);

quest(state, bagheera.instanceId);
console.log(
  `${bagheera.card.name} quests: gains ${bagheera.card.loreValue} lore (Player 1 lore now ${p1.lore})`,
);

playCharacter(state, beast.instanceId);
console.log(
  `${beast.card.name} enters play (cost ${beast.card.cost}, paid with ${beast.card.cost} ink) ` +
    `- can't quest or challenge this turn (just played)`,
);

console.log(`${princeEric.card.name} challenges ${tezuka.card.name}!`);
challenge(state, princeEric.instanceId, tezuka.instanceId);
const tezukaBanished = p2.discard.some((c) => c.instanceId === tezuka.instanceId);
console.log(
  `  ${princeEric.card.name}: ${princeEric.card.strength} strength vs ` +
    `${tezuka.card.name}: ${tezuka.card.willpower} willpower -> ${tezuka.damage} damage` +
    `${tezukaBanished ? ", LETHAL" : ""}`,
);
console.log(
  `  ${tezuka.card.name}: ${tezuka.card.strength} strength vs ${princeEric.card.name} -> ${princeEric.damage} damage`,
);
if (tezukaBanished) console.log(`  ${tezuka.card.name} is banished!`);
console.log(
  `  ${princeEric.card.name} survives with ${princeEric.damage} damage (${princeEric.card.willpower} willpower)\n`,
);

const p2HandBefore = p2.hand.length;
endTurn(state);
console.log(`End of turn ${state.turnNumber}. Player 2's turn begins.`);
console.log(`Player 2 draws a card (hand size: ${p2HandBefore} -> ${p2.hand.length})\n`);

console.log("Final state:");
console.log(`  Player 1 - lore: ${p1.lore}`);
for (const c of p1.play) {
  console.log(
    `    ${c.card.name}: ${c.exerted ? "exerted" : "ready"}, ${c.damage} damage` +
      `${c.playedThisTurn ? " (played this turn)" : ""}`,
  );
}
console.log(`  Player 2 - lore: ${p2.lore}, hand: ${p2.hand.length} card(s)`);
if (p2.discard.length > 0) {
  console.log(`    Discard: ${p2.discard.map((c) => c.card.name).join(", ")}`);
}
