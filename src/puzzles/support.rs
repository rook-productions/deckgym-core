//! Building blocks for the tactical puzzles: deck stubs, a clean mid-game position, and the
//! predicates that describe which action a puzzle accepts.

use crate::actions::SimpleAction;
use crate::card_ids::CardId;
use crate::database::get_card_by_enum;
use crate::models::{Card, EnergyType};
use crate::state::EnergyZone;
use crate::{Deck, State};

/// Number of cards every puzzle deck holds. Real decks are 20 cards, and the baseline value
/// function reads deck size, so keeping every puzzle at the same size stops deck size from
/// silently ranking one puzzle's actions differently from another's.
const PUZZLE_DECK_SIZE: usize = 20;

/// A 20-card deck built by repeating `ids` until it is full.
///
/// Puzzle decks are not legal decks (they ignore the 2-copy limit); they exist so the value
/// functions have a plausible "what could still come out of the deck" list for the Pokemon on
/// the board, and so `Professor's Research` and friends have something to draw.
pub(crate) fn puzzle_deck(ids: &[CardId], energy: &[EnergyType]) -> Deck {
    assert!(!ids.is_empty(), "a puzzle deck needs at least one card");
    // The Energy Zone cannot generate Colorless or Dragon, and `Game` panics on a deck that
    // declares one. MCTS builds a real `Game` from the puzzle position to run its playouts, so a
    // Colorless-Pokemon puzzle has to declare a type its attack costs can actually be paid with.
    assert!(
        energy.iter().all(|e| e.is_selectable()),
        "a puzzle deck may only declare Energy the Energy Zone can generate, got {energy:?}"
    );
    let cards: Vec<Card> = ids
        .iter()
        .cycle()
        .take(PUZZLE_DECK_SIZE)
        .map(|id| get_card_by_enum(*id))
        .collect();
    Deck {
        cards,
        energy_types: energy.to_vec(),
    }
}

/// A clean mid-game position: player 0 to move, turn 5 (so the first-two-turn restrictions on
/// evolving and on Rare Candy have lapsed), nothing played yet this turn.
///
/// Every puzzle then describes its own board, hands, points and Energy Zone on top of this.
pub(crate) fn position(deck_a: Deck, deck_b: Deck) -> State {
    let mut state = State::new(&deck_a, &deck_b);
    state.turn_count = 5;
    state.current_player = 0;
    state
}

/// An Energy Zone with this turn's Energy still available.
pub(crate) fn zone(energy: EnergyType) -> EnergyZone {
    EnergyZone {
        current: Some(energy),
        next: Some(energy),
    }
}

/// An Energy Zone whose Energy for this turn is already spent, so the puzzle offers no Attach
/// action. Used to keep a puzzle down to the choice it is actually about.
pub(crate) fn spent_zone(energy: EnergyType) -> EnergyZone {
    EnergyZone {
        current: None,
        next: Some(energy),
    }
}

/// One action a puzzle will accept, with the text used when reporting a failure.
pub struct Acceptable {
    describe: String,
    matcher: Box<dyn Fn(&SimpleAction) -> bool + Send + Sync>,
}

impl Acceptable {
    fn new(
        describe: String,
        matcher: impl Fn(&SimpleAction) -> bool + Send + Sync + 'static,
    ) -> Self {
        Acceptable {
            describe,
            matcher: Box::new(matcher),
        }
    }

    pub fn describe(&self) -> &str {
        &self.describe
    }

    pub fn matches(&self, action: &SimpleAction) -> bool {
        (self.matcher)(action)
    }
}

/// Attack with the attack of this name (whichever Pokemon is Active).
pub(crate) fn attack(title: &'static str) -> Acceptable {
    Acceptable::new(
        format!("Attack({title})"),
        move |action| matches!(action, SimpleAction::Attack(a) if a.title == title),
    )
}

/// Play the Trainer card of this name from hand.
pub(crate) fn play(card_name: &'static str) -> Acceptable {
    Acceptable::new(
        format!("Play({card_name})"),
        move |action| matches!(action, SimpleAction::Play { trainer_card } if trainer_card.name == card_name),
    )
}

/// Retreat the Active Pokemon, swapping in the Bench Pokemon at `idx`.
pub(crate) fn retreat_to(idx: usize) -> Acceptable {
    Acceptable::new(
        format!("Retreat({idx})"),
        move |action| matches!(action, SimpleAction::Retreat(i) if *i == idx),
    )
}

/// Attach this turn's Energy Zone Energy to the Pokemon at `idx` (0 is the Active Spot).
pub(crate) fn attach_to(idx: usize) -> Acceptable {
    Acceptable::new(format!("Attach(->{idx})"), move |action| match action {
        SimpleAction::Attach { attachments, .. } => {
            attachments.len() == 1 && attachments[0].2 == idx
        }
        _ => false,
    })
}

/// Evolve the Pokemon at `idx` into this card from hand.
pub(crate) fn evolve_at(idx: usize, into: CardId) -> Acceptable {
    let target = get_card_by_enum(into);
    let name = target.get_name();
    let id = target.get_id();
    Acceptable::new(format!("Evolve({name} at {idx})"), move |action| {
        matches!(action, SimpleAction::Evolve { evolution, in_play_idx, .. }
            if *in_play_idx == idx && evolution.get_id() == id)
    })
}

/// Put this Basic from hand onto any Bench slot.
pub(crate) fn place_on_bench(card_id: CardId) -> Acceptable {
    let card = get_card_by_enum(card_id);
    let name = card.get_name();
    let id = card.get_id();
    Acceptable::new(
        format!("Place({name} on the Bench)"),
        move |action| matches!(action, SimpleAction::Place(c, idx) if *idx != 0 && c.get_id() == id),
    )
}

/// Promote the Pokemon at `idx` into the Active Spot (the choice offered after a Knock Out).
pub(crate) fn activate(idx: usize) -> Acceptable {
    Acceptable::new(
        format!("Activate({idx})"),
        move |action| matches!(action, SimpleAction::Activate { in_play_idx, .. } if *in_play_idx == idx),
    )
}

/// Anything except evolving into this card. Used for the puzzles whose lesson is a move to
/// avoid rather than a single move to find.
pub(crate) fn anything_but_evolving_into(into: CardId) -> Acceptable {
    let target = get_card_by_enum(into);
    let name = target.get_name();
    let id = target.get_id();
    Acceptable::new(
        format!("anything except Evolve({name})"),
        move |action| !matches!(action, SimpleAction::Evolve { evolution, .. } if evolution.get_id() == id),
    )
}
