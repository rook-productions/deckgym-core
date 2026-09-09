//! Hand-built tactical positions where any decent player knows the move.
//!
//! Each puzzle is a concrete board (real cards, read from `database.json`), a one-line
//! justification, and the set of actions that count as solving it. A bot is asked for exactly one
//! action from the puzzle position and is graded on that action alone, so a puzzle only ever tests
//! the decision it is about.
//!
//! The positions live in `src/` rather than in `tests/` because two callers need them: the
//! `bot_puzzles` binary (`cargo run --release --bin bot_puzzles -- --players e`) and the test
//! target `tests/puzzles.rs`, which runs the same set against `e` and guards the baseline.
//!
//! Every card fact used below (HP, Energy cost, damage, Weakness, Retreat Cost, rules text) comes
//! from `database.json`, never from memory. The damage arithmetic is spelled out in each puzzle's
//! comment so a reader can check it against the card text without running anything.

pub mod support;

mod conditions;
mod development;
mod knockouts;
mod opponent_model;
mod safety;

use crate::actions::SimpleAction;
use crate::players::{create_players, PlayerCode};
use crate::State;
use rand::rngs::StdRng;
use rand::SeedableRng;
use support::Acceptable;

/// One tactical position and the actions that solve it.
pub struct Puzzle {
    /// Stable identifier, also the name used on the report line.
    pub name: &'static str,
    /// One line saying why the accepted action is the move.
    pub why: &'static str,
    /// The position, with player 0 to move.
    pub game: State,
    /// The actions that count as solved. Any one of them passes.
    pub acceptable: Vec<Acceptable>,
}

impl Puzzle {
    /// Whether `action` solves this puzzle.
    pub fn is_acceptable(&self, action: &SimpleAction) -> bool {
        self.acceptable.iter().any(|a| a.matches(action))
    }

    /// Human-readable form of the accepted set, for the report line.
    pub fn expectation(&self) -> String {
        self.acceptable
            .iter()
            .map(|a| a.describe().to_string())
            .collect::<Vec<_>>()
            .join(" or ")
    }

    /// The legal actions in the position, and who is to move.
    pub fn legal_actions(&self) -> (usize, Vec<crate::actions::Action>) {
        self.game.generate_possible_actions()
    }
}

/// Every puzzle, in a stable order.
pub fn all_puzzles() -> Vec<Puzzle> {
    let mut puzzles = Vec::new();
    puzzles.extend(knockouts::puzzles());
    puzzles.extend(safety::puzzles());
    puzzles.extend(conditions::puzzles());
    puzzles.extend(development::puzzles());
    puzzles.extend(opponent_model::puzzles());
    puzzles
}

/// What one bot did on one puzzle.
pub struct PuzzleOutcome {
    pub name: &'static str,
    pub why: &'static str,
    /// True when a strict majority of the trials chose an accepted action.
    pub passed: bool,
    /// Trials that chose an accepted action.
    pub votes: usize,
    pub trials: usize,
    /// The distinct actions chosen across the trials, most common first.
    pub chosen: Vec<String>,
    pub expected: String,
}

/// Asks one bot for one action from the puzzle position.
///
/// The bot is built with the puzzle's own decks so that value functions which look at "what could
/// still come out of my deck" see something coherent. `seed` drives the bot's RNG; several of the
/// bots consume randomness even when their choice is deterministic, so callers should run more
/// than one seed before believing a single answer.
pub fn solve(puzzle: &Puzzle, code: &PlayerCode, seed: u64) -> SimpleAction {
    assert!(
        !matches!(code, PlayerCode::H),
        "the human player reads stdin and cannot be benchmarked"
    );
    let (actor, actions) = puzzle.legal_actions();
    assert_eq!(
        actor, 0,
        "puzzle {} must be player 0's decision",
        puzzle.name
    );
    assert!(
        actions.len() > 1,
        "puzzle {} offers no choice at all",
        puzzle.name
    );
    let mut players = create_players(
        puzzle.game.decks[0].clone(),
        puzzle.game.decks[1].clone(),
        vec![code.clone(), PlayerCode::R],
    );
    let mut rng = StdRng::seed_from_u64(seed);
    players[0]
        .decision_fn(&mut rng, &puzzle.game, &actions)
        .action
}

/// Runs one puzzle `trials` times and takes the majority verdict.
///
/// The coin-dependent bots (`r`, `w`, `m`) do not answer the same way twice, and even the search
/// bots draw from the RNG while expanding chance nodes, so a single sample would report noise.
pub fn run_puzzle(puzzle: &Puzzle, code: &PlayerCode, trials: usize) -> PuzzleOutcome {
    assert!(trials > 0, "a puzzle needs at least one trial");
    let mut votes = 0;
    let mut tally: Vec<(String, usize)> = Vec::new();
    for seed in 0..trials as u64 {
        let action = solve(puzzle, code, seed);
        if puzzle.is_acceptable(&action) {
            votes += 1;
        }
        let label = action.to_string();
        match tally.iter_mut().find(|(seen, _)| *seen == label) {
            Some((_, count)) => *count += 1,
            None => tally.push((label, 1)),
        }
    }
    tally.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    PuzzleOutcome {
        name: puzzle.name,
        why: puzzle.why,
        passed: votes * 2 > trials,
        votes,
        trials,
        chosen: tally
            .into_iter()
            .map(|(label, count)| format!("{label} x{count}"))
            .collect(),
        expected: puzzle.expectation(),
    }
}

/// Runs every puzzle for one bot.
pub fn run_all(code: &PlayerCode, trials: usize) -> Vec<PuzzleOutcome> {
    all_puzzles()
        .iter()
        .map(|puzzle| run_puzzle(puzzle, code, trials))
        .collect()
}

/// How many puzzles a bot solved.
pub fn pass_count(outcomes: &[PuzzleOutcome]) -> usize {
    outcomes.iter().filter(|o| o.passed).count()
}

/// How many of a puzzle's legal actions it accepts, and how many legal actions there are.
pub fn acceptance(puzzle: &Puzzle) -> (usize, usize) {
    let (_, actions) = puzzle.legal_actions();
    let accepted = actions
        .iter()
        .filter(|a| puzzle.is_acceptable(&a.action))
        .count();
    (accepted, actions.len())
}

/// The score a chooser picking uniformly at random would post, in expectation, under the same
/// majority-of-`trials` rule the bots are graded by.
///
/// Puzzles are small positions, so several of them offer only two or three legal actions and a
/// coin-flipping chooser gets a real share of them. This number is the floor a bot has to clear
/// before its score means anything; zero is not the floor. `r` lands near it by construction.
pub fn chance_score(trials: usize) -> f64 {
    assert!(trials > 0, "a puzzle needs at least one trial");
    // `run_puzzle` passes a puzzle on `votes * 2 > trials`, so the winning post is this many.
    let needed = trials / 2 + 1;
    all_puzzles()
        .iter()
        .map(|puzzle| {
            let (accepted, legal) = acceptance(puzzle);
            let p = accepted as f64 / legal as f64;
            (needed..=trials)
                .map(|k| {
                    binomial(trials, k) * p.powi(k as i32) * (1.0 - p).powi((trials - k) as i32)
                })
                .sum::<f64>()
        })
        .sum()
}

/// `n` choose `k`, computed multiplicatively so the intermediate values stay small.
fn binomial(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}
