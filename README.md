# lorcana-sim

Deck simulator for Disney Lorcana: plays a configurable number of games
between two decks using heuristic bots and reports win rate, game length,
lore-per-turn, and other matchup stats.

For-fun / learning project. Scope is intentionally limited to whatever
cards appear in the target decks being tested, not full card-pool coverage.
See `docs/scope.md` for the full rationale and architecture notes.

## Phase plan

1. Pull and structure card data for target decks only
2. Categorize each card's ability (generic keyword vs. bespoke effect)
3. Build core engine (zones, turn structure, event hooks) against a small
   vanilla-heavy card subset
4. Implement bespoke abilities for the target decks
5. Build a heuristic bot with archetype-based strategy
6. Build the simulation runner + stats output
7. Only then: expand to more decks/cards if it's still fun

## Setup

```
npm install
npm run fetch-cards   # pulls the LorcanaJSON dump into data/cards/raw
npm run filter-cards  # filters it down to data/cards/cards.json based on data/decks/*.json
npm run sim -- <deckA> <deckB> --games 1000
```
