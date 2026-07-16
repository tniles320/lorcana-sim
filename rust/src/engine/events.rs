use std::sync::Mutex;

/// Unlike the TS event bus (which passes live object references in a loose
/// `any` payload), each variant here carries owned/cloned data only. Rust's
/// ownership rules make it impractical to stash borrowed references to game
/// state inside a globally-stored handler closure, so handlers get ids and
/// values instead and can look things up in `GameState` themselves if needed.
#[derive(Debug, Clone)]
pub enum Event {
    StartOfTurn {
        active_player: usize,
    },
    EndOfTurn {
        active_player: usize,
    },
    Play {
        player_id: String,
        instance_id: String,
    },
    Quest {
        player_id: String,
        instance_id: String,
        lore_gained: i32,
    },
    Challenge {
        attacker_id: String,
        defender_id: String,
        damage_to_defender: i32,
        damage_to_attacker: i32,
    },
    Banish {
        owner_id: String,
        instance_id: String,
    },
    Ink {
        player_id: String,
        instance_id: String,
    },
}

type Handler = Box<dyn FnMut(&Event) + Send>;

static HANDLERS: Mutex<Vec<Handler>> = Mutex::new(Vec::new());

pub fn on(handler: Handler) {
    HANDLERS.lock().unwrap().push(handler);
}

pub fn emit(event: &Event) {
    for handler in HANDLERS.lock().unwrap().iter_mut() {
        handler(event);
    }
}

/// Test-only: clears all registered handlers between isolated test runs.
pub fn clear_handlers() {
    HANDLERS.lock().unwrap().clear();
}
