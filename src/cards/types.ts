export type Ink =
  | "Amber"
  | "Amethyst"
  | "Emerald"
  | "Ruby"
  | "Sapphire"
  | "Steel";

export type CardType = "Character" | "Action" | "Item" | "Location";

export interface Card {
  id: string;
  name: string;
  version: string | null;
  type: CardType[];
  ink: Ink;
  cost: number;
  inkwell: boolean;
  classifications: string[];
  text: string;
  keywords: string[];
  strength: number | null;
  willpower: number | null;
  loreValue: number | null;
  moveCost: number | null;
  rarity: string;
  setCode: string;
}

export interface DeckEntry {
  name: string;
  version: string;
  count: number;
}

export interface Deck {
  name: string;
  description?: string;
  cards: DeckEntry[];
}
