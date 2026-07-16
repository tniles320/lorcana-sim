use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Ink {
    Amber,
    Amethyst,
    Emerald,
    Ruby,
    Sapphire,
    Steel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum CardType {
    Character,
    Action,
    Item,
    Location,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    #[serde(rename = "type")]
    pub card_type: Vec<CardType>,
    pub ink: Ink,
    pub cost: i32,
    pub inkwell: bool,
    pub classifications: Vec<String>,
    pub text: String,
    pub keywords: Vec<String>,
    pub strength: Option<i32>,
    pub willpower: Option<i32>,
    pub lore_value: Option<i32>,
    pub move_cost: Option<i32>,
    pub rarity: String,
    pub set_code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeckEntry {
    pub name: String,
    pub version: String,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Deck {
    pub name: String,
    pub description: Option<String>,
    pub cards: Vec<DeckEntry>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cards_file() -> PathBuf {
    manifest_dir().join("../data/cards/cards.json")
}

fn decks_dir() -> PathBuf {
    manifest_dir().join("../data/decks")
}

pub fn load_card_index() -> HashMap<(String, String), Card> {
    let raw = fs::read_to_string(cards_file()).expect("failed to read cards.json");
    let cards: Vec<Card> = serde_json::from_str(&raw).expect("failed to parse cards.json");
    cards
        .into_iter()
        .map(|c| ((c.name.clone(), c.version.clone().unwrap_or_default()), c))
        .collect()
}

/// Looks up a single card by name and version from cards.json.
pub fn find_card(name: &str, version: &str) -> Card {
    let index = load_card_index();
    index
        .get(&(name.to_string(), version.to_string()))
        .unwrap_or_else(|| panic!("Card not found in cards.json: {name} - {version}"))
        .clone()
}

/// Loads a deck file's cards, expanded to one entry per physical copy.
pub fn load_deck(deck_file_name: &str) -> Vec<Card> {
    let index = load_card_index();
    let path = decks_dir().join(deck_file_name);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("failed to read deck file {:?}", path));
    let deck: Deck = serde_json::from_str(&raw).expect("failed to parse deck file");

    let mut cards = Vec::new();
    for entry in &deck.cards {
        let key = (entry.name.clone(), entry.version.clone());
        let card = index.get(&key).unwrap_or_else(|| {
            panic!(
                "Card not found in cards.json: {} - {}",
                entry.name, entry.version
            )
        });
        for _ in 0..entry.count {
            cards.push(card.clone());
        }
    }
    cards
}
