/**
 * Fixed, non-random scenario run identically by the TS and Rust engines
 * (see rust/examples/scenario.rs) to verify they agree bit-for-bit on real
 * game logic. No shuffling is involved — the point is to check the ported
 * rules, not two independent PRNGs.
 *
 * Run: npx tsx parity/scenario.ts
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
// Simulates having exerted on a prior turn (via Bodyguard entry or questing),
// so it's a legal challenge target now.
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

quest(state, bagheera.instanceId);
playCharacter(state, beast.instanceId);
challenge(state, princeEric.instanceId, tezuka.instanceId);
endTurn(state);

const summary = {
  turnNumber: state.turnNumber,
  activePlayer: state.activePlayer,
  phase: state.phase,
  player1: {
    lore: state.players[0].lore,
    play: state.players[0].play
      .map((c) => ({
        name: c.card.name,
        exerted: c.exerted,
        damage: c.damage,
        playedThisTurn: c.playedThisTurn,
      }))
      .sort((a, b) => a.name.localeCompare(b.name)),
  },
  player2: {
    lore: state.players[1].lore,
    handSize: state.players[1].hand.length,
    discard: state.players[1].discard.map((c) => c.card.name).sort(),
  },
};

console.log(JSON.stringify(summary, null, 2));
