//! The tactical puzzle benchmark (bot plan step 0, item 3).
//!
//! The positions themselves live in `src/puzzles/` because the `bot_puzzles` binary needs them
//! too; this target is the part that runs in CI. `well_formed_test` checks that every puzzle is a
//! real decision, and `expectiminimax_baseline_test` pins the default bot's score so a change to
//! the engine or to the value function cannot quietly make it worse.
#[path = "puzzles/expectiminimax_baseline_test.rs"]
mod expectiminimax_baseline_test;
#[path = "puzzles/well_formed_test.rs"]
mod well_formed_test;
