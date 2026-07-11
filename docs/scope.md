# Lorcana Deck Simulator — Project Scope & Architecture Notes

## Goal

Build a simulator that takes two decks (custom or prebuilt), plays a configurable number of games between them using heuristic-driven bots, and outputs performance stats (win rate, average game length, lore-per-turn, etc.).

This is a for-fun / learning project. Scope is intentionally limited to **cards currently in the target decks**, not full card-pool coverage. New sets and full rules coverage are explicitly out of scope for now — this is a "make it work for a handful of real decks" project, not a production TCG engine.

## Phase order (recommended)

1. Pull and structure card data for target decks only
2. Categorize each card's ability (generic keyword vs. bespoke effect)
3. Build core engine (zones, turn structure, event hooks) against a small vanilla-heavy card subset
4. Implement bespoke abilities for the target decks
5. Build a heuristic bot with archetype-based strategy
6. Build the simulation runner + stats output
7. Only then: expand to more decks/cards if it's still fun

---

## 1. Data source

Two viable free APIs, no key required for either:

- **Lorcast API** (`api.lorcast.com`) — REST API, returns ink, cost, strength/willpower, keywords, classifications, legalities, and rules text as a plain string per card. Beta but stated safe for production use. Rate limit guidance: ~10 req/sec (50-100ms between requests).
- **LorcanaJSON** (lorcanajson.org) — downloadable structured JSON dump of the full card pool, sourced from the same data as the official Lorcana app. Good option if you'd rather have one static file to load and version locally than hit an API repeatedly.

**Recommendation:** Pull once, cache locally as JSON (e.g. `data/cards.json`), scoped to just the cards appearing in your target decks. Don't build a live-fetch dependency into the sim loop — treat the card database as a static local asset you regenerate occasionally.

## 2. Ability taxonomy

Before writing engine code, sort every card in your target decks into buckets. This determines how much of the "bespoke effect" problem you actually have for *this* scope (likely much smaller than the full card pool suggests).

- **Generic keywords** — Bodyguard, Rush, Ward, Support, Challenger +N, Resist +N, Evasive, Singer N — these can be implemented once as reusable modifiers/flags on a card, not per-card logic.
- **Bespoke triggered/static effects** — "whenever this character quests, do X," "when you play this character, do Y" — these need individual implementation but often cluster into small families (draw a card, deal damage, remove damage, gain lore, move a card between zones) that can share underlying primitives.

Practical next step: for each card in your target decks, tag it as `keyword`, `bespoke-common-pattern`, or `bespoke-unique`. This gives an honest estimate of implementation effort before any engine code is written.

## 3. Engine state model

Rough shape to hand to Claude Code as a starting point (not final — expect it to evolve once real cards are in front of it):

**Zones per player:**
- Deck (ordered, hidden)
- Hand (hidden from opponent)
- Play (characters/items/locations in play)
- Inkwell (cards committed as ink — track exerted/available separately from count)
- Discard

**Turn structure (phases):**
1. Ready — untap/ready exerted cards
2. Set — start-of-turn triggers resolve
3. Draw
4. Main — play cards, ink a card, quest, challenge, sing songs, activate abilities, in any order/combination until pass

**Core event hooks abilities attach to** (most bespoke effects will hang off one of these):
- `onPlay` (character/item/action enters play)
- `onQuest`
- `onChallenge` (both as attacker and as defender)
- `onStartOfTurn` / `onEndOfTurn`
- `onBanish` (character leaves play via damage/effect)
- `onInk` (card moved to inkwell)

Designing abilities as data (a list of trigger + effect-primitive pairs per card) rather than one-off functions per card will pay off — even for a small scope, it keeps the bespoke cards from turning into unmaintainable spaghetti.

## 4. Strategy / bot heuristics

Framing to build from: **archetype-baseline heuristics with matchup-conditional adjustments**, not a general-purpose search algorithm. This matches how the decks actually get played and keeps the bot's logic legible and tunable.

Example structure (Steel/Sapphire ramp-style deck):
- Baseline priority: ink every turn if possible; prioritize playing ramp/draw pieces early; target a specific ink count by a specific turn as a soft goal.
- Default combat posture: avoid unnecessary challenges early, hold characters back for board control once ramp target is hit.
- **Conditional adjustment vs. aggro opponent:** lower the ramp-turn target, prioritize willpower/blockers earlier, be more willing to trade in combat to slow opponent's clock.
- **Conditional adjustment vs. control/ramp opponent:** lean into racing lore instead of holding back.

Implementation approach: give the bot a scoring function over legal moves each decision point (e.g. weighted sum of "lore gained," "board state improvement," "tempo," "risk of losing character") with weights that shift based on a simple opponent-archetype read (inferred from deck composition, not hidden information). Start with something crude and tune once you see simulated games play out — this is the kind of thing that's much easier to iterate on inside Claude Code with real game logs in front of you than to fully design up front.

## 5. Output / stats

Minimum useful output per deck-vs-deck simulation batch:
- Win rate (with game count and a rough confidence sense — e.g. don't trust 20-game samples)
- Average game length (turns)
- Average lore-per-turn for each side
- Optionally: which cards ended up unplayed/dead most often (useful deckbuilding signal)

---

## Handoff note

This doc is a starting scope, not a spec set in stone — expect the engine design in particular to shift once real card data and real abilities are in front of you. Suggested first session goals: (1) pull card data for the actual target decks, (2) run the taxonomy pass from section 2 to get a real effort estimate, (3) stub the zone/turn-structure skeleton from section 3 against a few vanilla cards to prove the loop works before touching bespoke abilities.
