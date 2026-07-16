import { readFile, writeFile, readdir } from "node:fs/promises";
import path from "node:path";
import type { Card, Deck } from "../src/cards/types.ts";

const RAW_FILE = path.resolve(
  import.meta.dirname,
  "../data/cards/raw/all-cards.json",
);
const DECKS_DIR = path.resolve(import.meta.dirname, "../data/decks");
const OUT_FILE = path.resolve(import.meta.dirname, "../data/cards/cards.json");

interface LorcastCard {
  id: string;
  name: string;
  version: string | null;
  type: string[];
  ink: string;
  cost: number;
  inkwell: boolean;
  classifications: string[] | null;
  text: string | null;
  keywords: string[] | null;
  strength: number | null;
  willpower: number | null;
  lore: number | null;
  move_cost: number | null;
  rarity: string;
  set: { code: string };
}

function toCard(raw: LorcastCard): Card {
  return {
    id: raw.id,
    name: raw.name,
    version: raw.version,
    type: raw.type as Card["type"],
    ink: raw.ink as Card["ink"],
    cost: raw.cost,
    inkwell: raw.inkwell,
    classifications: raw.classifications ?? [],
    text: (raw.text ?? "").replace(/\r\n/g, "\n").trim(),
    keywords: raw.keywords ?? [],
    strength: raw.strength,
    willpower: raw.willpower,
    loreValue: raw.lore,
    moveCost: raw.move_cost,
    rarity: raw.rarity,
    setCode: raw.set.code,
  };
}

async function loadDecks(): Promise<Deck[]> {
  const files = (await readdir(DECKS_DIR)).filter((f) => f.endsWith(".json"));
  const decks: Deck[] = [];
  for (const file of files) {
    const raw = await readFile(path.join(DECKS_DIR, file), "utf-8");
    decks.push(JSON.parse(raw));
  }
  return decks;
}

async function main() {
  const rawCards: LorcastCard[] = JSON.parse(
    await readFile(RAW_FILE, "utf-8"),
  );
  const byNameVersion = new Map<string, LorcastCard>();
  for (const card of rawCards) {
    byNameVersion.set(`${card.name}|${card.version}`, card);
  }

  const decks = await loadDecks();
  const needed = new Map<string, Card>();
  const missing: string[] = [];

  for (const deck of decks) {
    for (const entry of deck.cards) {
      const key = `${entry.name}|${entry.version}`;
      const raw = byNameVersion.get(key);
      if (!raw) {
        missing.push(`${deck.name}: ${entry.name} - ${entry.version}`);
        continue;
      }
      needed.set(raw.id, toCard(raw));
    }
  }

  if (missing.length > 0) {
    console.error("Could not find these cards in the raw card data:");
    for (const m of missing) console.error(`  ${m}`);
    process.exit(1);
  }

  const cards = Array.from(needed.values()).sort((a, b) =>
    a.name.localeCompare(b.name),
  );
  await writeFile(OUT_FILE, JSON.stringify(cards, null, 2));
  console.log(
    `wrote ${cards.length} unique cards (from ${decks.length} decks) to ${OUT_FILE}`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
