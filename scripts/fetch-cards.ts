import { writeFile, mkdir } from "node:fs/promises";
import path from "node:path";

const LORCAST_BASE = "https://api.lorcast.com/v0";
const CARD_TYPES = ["character", "action", "item", "location"] as const;
const OUT_DIR = path.resolve(import.meta.dirname, "../data/cards/raw");
const OUT_FILE = path.join(OUT_DIR, "all-cards.json");

interface LorcastCard {
  id: string;
  [key: string]: unknown;
}

async function fetchType(type: string): Promise<LorcastCard[]> {
  const url = `${LORCAST_BASE}/cards/search?q=${encodeURIComponent(`type:${type}`)}`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Lorcast request failed for type:${type} (${res.status})`);
  }
  const body = (await res.json()) as { results: LorcastCard[] };
  return body.results;
}

async function main() {
  const byId = new Map<string, LorcastCard>();

  for (const type of CARD_TYPES) {
    const cards = await fetchType(type);
    for (const card of cards) {
      byId.set(card.id, card);
    }
    console.log(`fetched ${cards.length} ${type} cards`);
    // stay well under Lorcast's ~10 req/sec guidance
    await new Promise((resolve) => setTimeout(resolve, 150));
  }

  await mkdir(OUT_DIR, { recursive: true });
  const all = Array.from(byId.values());
  await writeFile(OUT_FILE, JSON.stringify(all, null, 2));
  console.log(`wrote ${all.length} total cards to ${OUT_FILE}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
