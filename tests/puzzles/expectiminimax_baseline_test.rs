//! The baseline the rest of the bot work is measured against.
//!
//! `e` (ExpectiMiniMax, depth 3) is the simulator's default player and the bot the published
//! matchup matrix uses, so its puzzle score is the number every challenger has to beat. The
//! assertion below is a floor, not an equality: solving more puzzles is the point of the work, and
//! a run that solves fewer means something regressed in the engine, in the move generator, or in
//! the value function. Re-measure and raise the floor deliberately when a change is meant to
//! improve it; never lower it to make a red test green.

use deckgym::players::PlayerCode;
use deckgym::puzzles::{pass_count, run_all};

/// Measured 2026-09-09T17:15Z on `pocket-lab`: 22 of 27 with 5 trials per puzzle, against a
/// uniform-random floor of 7.2. All five misses are positions that only resolve once you look at
/// the opponent's reply: `do_not_attack_into_the_lethal_rocky_helmet`,
/// `retreat_the_ex_out_of_the_game_losing_knockout`, `do_not_evolve_into_the_two_point_target`,
/// `attach_to_the_bench_attacker_not_the_dead_active` and `keep_the_damaged_ex_off_the_active_spot`.
/// `e` stops searching at the first opponent-owned state, so it cannot see any of them; closing
/// that gap is what steps 1 to 3 of the bot plan are for.
const EXPECTIMINIMAX_BASELINE: usize = 22;

/// Trials per puzzle. `e` expands chance nodes with the shared RNG, so its answer is not strictly
/// a function of the position; five samples and a majority verdict keep the test from flapping.
const TRIALS: usize = 5;

#[test]
fn puzzles_baseline_for_expectiminimax() {
    let outcomes = run_all(&PlayerCode::E { max_depth: 3 }, TRIALS);
    let passed = pass_count(&outcomes);
    let failed: Vec<&str> = outcomes
        .iter()
        .filter(|o| !o.passed)
        .map(|o| o.name)
        .collect();
    assert!(
        passed >= EXPECTIMINIMAX_BASELINE,
        "e solved {passed}/{} puzzles, below the recorded baseline of {EXPECTIMINIMAX_BASELINE}. Failed: {failed:?}",
        outcomes.len()
    );
}
