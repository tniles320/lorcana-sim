use crate::cards::Card;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Set,
    Draw,
    Main,
}

#[derive(Debug, Clone)]
pub struct CardInstance {
    pub instance_id: String,
    pub card: Card,
    pub damage: i32,
    pub exerted: bool,
    /// True from the moment this character enters play until this player's next Ready phase ("wet ink").
    pub played_this_turn: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub id: String,
    /// Ordered, hidden. Top of deck is the last element (drawn via pop).
    pub deck: Vec<CardInstance>,
    pub hand: Vec<CardInstance>,
    pub play: Vec<CardInstance>,
    pub inkwell: Vec<CardInstance>,
    pub discard: Vec<CardInstance>,
    pub lore: i32,
    pub inked_this_turn: bool,
}

pub struct GameState {
    pub players: [PlayerState; 2],
    pub turn_number: u32,
    /// Always 0 or 1 — indexes into `players`.
    pub active_player: usize,
    pub phase: Phase,
    /// Set once the game has ended. `None` means still in progress.
    pub game_over: Option<GameOver>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOverReason {
    LoreVictory,
    /// A player was required to draw and their deck was empty.
    DeckOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameOver {
    pub winner: usize,
    pub reason: GameOverReason,
}

pub const LORE_TO_WIN: i32 = 20;

/// Checks whether either player has reached the lore threshold and, if so,
/// records it on `state.game_over`. Idempotent — does nothing if the game
/// has already ended (e.g. by deck-out, set elsewhere in `turn.rs`).
pub fn check_lore_victory(state: &mut GameState) {
    if state.game_over.is_some() {
        return;
    }
    for (i, player) in state.players.iter().enumerate() {
        if player.lore >= LORE_TO_WIN {
            state.game_over = Some(GameOver {
                winner: i,
                reason: GameOverReason::LoreVictory,
            });
            return;
        }
    }
}

static INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_instance_id() -> String {
    let n = INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("inst_{n}")
}

/// Wraps a static card definition into a fresh in-game instance (own id, no damage, ready).
pub fn create_instance(card: Card) -> CardInstance {
    CardInstance {
        instance_id: next_instance_id(),
        card,
        damage: 0,
        exerted: false,
        played_this_turn: false,
    }
}

/// Fisher-Yates shuffle, driven by an injectable `rng` closure returning a
/// value in [0, 1) — mirrors the TS engine's `shuffle` so both can be handed
/// the same seeded sequence for cross-language determinism checks.
pub fn shuffle<T: Clone>(items: &[T], rng: &mut impl FnMut() -> f64) -> Vec<T> {
    let mut arr = items.to_vec();
    for i in (1..arr.len()).rev() {
        let j = (rng() * (i as f64 + 1.0)).floor() as usize;
        arr.swap(i, j);
    }
    arr
}

const OPENING_HAND_SIZE: usize = 7;

/// Moves the top card of the deck into hand (no clone — same instance moves,
/// mirroring the TS version's `deck.pop()` into `hand.push()`).
pub fn draw_card(player: &mut PlayerState) -> Option<String> {
    let card = player.deck.pop()?;
    let instance_id = card.instance_id.clone();
    player.hand.push(card);
    Some(instance_id)
}

fn create_player(id: &str, deck_cards: Vec<Card>, rng: &mut impl FnMut() -> f64) -> PlayerState {
    let shuffled = shuffle(&deck_cards, rng);
    let deck = shuffled.into_iter().map(create_instance).collect();
    PlayerState {
        id: id.to_string(),
        deck,
        hand: Vec::new(),
        play: Vec::new(),
        inkwell: Vec::new(),
        discard: Vec::new(),
        lore: 0,
        inked_this_turn: false,
    }
}

/// Builds initial state: shuffled decks and opening hands drawn, phase not yet advanced.
pub fn create_game(
    deck_a: Vec<Card>,
    deck_b: Vec<Card>,
    rng: &mut impl FnMut() -> f64,
) -> GameState {
    let mut player_a = create_player("player-1", deck_a, rng);
    let mut player_b = create_player("player-2", deck_b, rng);

    for _ in 0..OPENING_HAND_SIZE {
        draw_card(&mut player_a);
        draw_card(&mut player_b);
    }

    GameState {
        players: [player_a, player_b],
        turn_number: 1,
        active_player: 0,
        phase: Phase::Ready,
        game_over: None,
    }
}

/// A non-deterministic rng closure backed by `rand`, for real (non-test) games.
pub fn system_rng() -> impl FnMut() -> f64 {
    let mut rng = rand::thread_rng();
    move || rng.r#gen::<f64>()
}

pub fn opponent_index(player_index: usize) -> usize {
    if player_index == 0 {
        1
    } else {
        0
    }
}
