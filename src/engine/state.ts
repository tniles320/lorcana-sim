import type { Card } from "../cards/types.ts";

export type Phase = "ready" | "set" | "draw" | "main";

export interface CardInstance {
  instanceId: string;
  card: Card;
  damage: number;
  exerted: boolean;
  /** True from the moment this character enters play until this player's next Ready phase ("wet ink"). */
  playedThisTurn: boolean;
}

export interface PlayerState {
  id: string;
  /** Ordered, hidden. Top of deck is the last element (drawn via pop). */
  deck: CardInstance[];
  hand: CardInstance[];
  play: CardInstance[];
  inkwell: CardInstance[];
  discard: CardInstance[];
  lore: number;
  inkedThisTurn: boolean;
}

export interface GameState {
  players: [PlayerState, PlayerState];
  turnNumber: number;
  activePlayer: 0 | 1;
  phase: Phase;
}

let instanceCounter = 0;
function nextInstanceId(): string {
  instanceCounter += 1;
  return `inst_${instanceCounter}`;
}

/** Wraps a static card definition into a fresh in-game instance (own id, no damage, ready). */
export function createInstance(card: Card): CardInstance {
  return {
    instanceId: nextInstanceId(),
    card,
    damage: 0,
    exerted: false,
    playedThisTurn: false,
  };
}

export function shuffle<T>(items: T[], rng: () => number = Math.random): T[] {
  const arr = [...items];
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

const OPENING_HAND_SIZE = 7;

export function drawCard(player: PlayerState): CardInstance | undefined {
  const card = player.deck.pop();
  if (card) player.hand.push(card);
  return card;
}

function createPlayer(
  id: string,
  deckCards: Card[],
  rng?: () => number,
): PlayerState {
  const deck = shuffle(deckCards, rng).map(createInstance);
  return {
    id,
    deck,
    hand: [],
    play: [],
    inkwell: [],
    discard: [],
    lore: 0,
    inkedThisTurn: false,
  };
}

export interface CreateGameOptions {
  rng?: () => number;
}

/** Builds initial state: shuffled decks and opening hands drawn, phase not yet advanced. */
export function createGame(
  deckA: Card[],
  deckB: Card[],
  options: CreateGameOptions = {},
): GameState {
  const players: [PlayerState, PlayerState] = [
    createPlayer("player-1", deckA, options.rng),
    createPlayer("player-2", deckB, options.rng),
  ];
  for (const player of players) {
    for (let i = 0; i < OPENING_HAND_SIZE; i++) drawCard(player);
  }
  return {
    players,
    turnNumber: 1,
    activePlayer: 0,
    phase: "ready",
  };
}

export function activePlayer(state: GameState): PlayerState {
  return state.players[state.activePlayer];
}

export function opponentOf(state: GameState, playerIndex: 0 | 1): PlayerState {
  return state.players[playerIndex === 0 ? 1 : 0];
}
