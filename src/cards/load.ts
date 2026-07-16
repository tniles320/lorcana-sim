import { readFileSync } from "node:fs";
import path from "node:path";
import type { Card, Deck } from "./types.ts";

const CARDS_FILE = path.resolve(
  import.meta.dirname,
  "../../data/cards/cards.json",
);
const DECKS_DIR = path.resolve(import.meta.dirname, "../../data/decks");

let cardIndex: Map<string, Card> | null = null;

function loadCardIndex(): Map<string, Card> {
  if (!cardIndex) {
    const cards: Card[] = JSON.parse(readFileSync(CARDS_FILE, "utf-8"));
    cardIndex = new Map(cards.map((c) => [`${c.name}|${c.version}`, c]));
  }
  return cardIndex;
}

/** Loads a deck file's cards, expanded to one entry per physical copy. */
export function loadDeck(deckFileName: string): Card[] {
  const raw: Deck = JSON.parse(
    readFileSync(path.join(DECKS_DIR, deckFileName), "utf-8"),
  );
  const index = loadCardIndex();
  const cards: Card[] = [];
  for (const entry of raw.cards) {
    const card = index.get(`${entry.name}|${entry.version}`);
    if (!card) {
      throw new Error(
        `Card not found in cards.json: ${entry.name} - ${entry.version}`,
      );
    }
    for (let i = 0; i < entry.count; i++) cards.push(card);
  }
  return cards;
}
