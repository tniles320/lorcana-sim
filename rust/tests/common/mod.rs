// Shared across multiple test binaries (state_test.rs, turn_test.rs,
// actions_test.rs); each one only uses a subset of this API, so Rust's
// per-binary dead-code analysis would otherwise warn in the others.
#![allow(dead_code)]

use lorcana_sim::cards::{Card, CardType, Ink};
use lorcana_sim::engine::state::CardInstance;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);
static INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Builder for test fixture cards, mirroring the TS suite's `makeCard(overrides)`
/// helper — Rust has no object-literal-with-defaults, so this uses the
/// standard builder pattern instead (`CardBuilder::new().cost(3).build()`).
pub struct CardBuilder {
    card: Card,
}

impl CardBuilder {
    pub fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        CardBuilder {
            card: Card {
                id: format!("test_{n}"),
                name: format!("Test Character {n}"),
                version: Some("Test Version".to_string()),
                card_type: vec![CardType::Character],
                ink: Ink::Amber,
                cost: 1,
                inkwell: true,
                classifications: Vec::new(),
                text: String::new(),
                keywords: Vec::new(),
                strength: Some(1),
                willpower: Some(1),
                lore_value: Some(1),
                move_cost: None,
                rarity: "Common".to_string(),
                set_code: "1".to_string(),
            },
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.card.name = name.to_string();
        self
    }

    pub fn cost(mut self, cost: i32) -> Self {
        self.card.cost = cost;
        self
    }

    pub fn inkwell(mut self, inkwell: bool) -> Self {
        self.card.inkwell = inkwell;
        self
    }

    pub fn strength(mut self, strength: i32) -> Self {
        self.card.strength = Some(strength);
        self
    }

    pub fn willpower(mut self, willpower: i32) -> Self {
        self.card.willpower = Some(willpower);
        self
    }

    pub fn lore_value(mut self, lore_value: i32) -> Self {
        self.card.lore_value = Some(lore_value);
        self
    }

    pub fn keywords(mut self, keywords: &[&str]) -> Self {
        self.card.keywords = keywords.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.card.text = text.to_string();
        self
    }

    pub fn build(self) -> Card {
        self.card
    }

    /// Wraps the built card into a fresh in-game `CardInstance`, for pushing
    /// directly into a player's hand/play/inkwell/discard in a test.
    pub fn build_instance(self) -> CardInstance {
        let n = INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        CardInstance {
            instance_id: format!("test_inst_{n}"),
            card: self.card,
            damage: 0,
            exerted: false,
            played_this_turn: false,
        }
    }
}
