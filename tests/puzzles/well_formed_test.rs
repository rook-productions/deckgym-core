//! A puzzle is only worth counting if it is a genuine decision: player 0 to move, more than one
//! legal action, at least one of them accepted, and at least one of them not accepted. These
//! checks catch the usual way a hand-built position rots, which is an engine change quietly
//! removing (or adding) an action so that the puzzle passes for the wrong reason.

use deckgym::puzzles::all_puzzles;
use std::collections::HashSet;

#[test]
fn puzzles_are_well_formed_decisions() {
    let puzzles = all_puzzles();
    assert!(
        puzzles.len() >= 24,
        "the plan asks for 24 to 30 puzzles, found {}",
        puzzles.len()
    );
    assert!(
        puzzles.len() <= 30,
        "the plan asks for 24 to 30 puzzles, found {}",
        puzzles.len()
    );

    for puzzle in &puzzles {
        let (actor, actions) = puzzle.legal_actions();
        assert_eq!(actor, 0, "{}: player 0 should be to move", puzzle.name);
        assert!(
            actions.len() > 1,
            "{}: only one legal action, so nothing is being tested",
            puzzle.name
        );

        let accepted = actions
            .iter()
            .filter(|a| puzzle.is_acceptable(&a.action))
            .count();
        assert!(
            accepted > 0,
            "{}: none of the {} legal actions is accepted (wanted {}), so no bot can pass it. Legal: {:?}",
            puzzle.name,
            actions.len(),
            puzzle.expectation(),
            actions.iter().map(|a| a.action.to_string()).collect::<Vec<_>>()
        );
        assert!(
            accepted < actions.len(),
            "{}: every legal action is accepted, so no bot can fail it",
            puzzle.name
        );
    }
}

#[test]
fn puzzle_names_are_unique() {
    let puzzles = all_puzzles();
    let names: HashSet<&str> = puzzles.iter().map(|p| p.name).collect();
    assert_eq!(
        names.len(),
        puzzles.len(),
        "puzzle names are the report's identifiers and must not repeat"
    );
}

#[test]
fn puzzles_explain_themselves() {
    for puzzle in all_puzzles() {
        assert!(
            !puzzle.why.is_empty(),
            "{}: every puzzle needs its one-line justification",
            puzzle.name
        );
        assert!(
            !puzzle.acceptable.is_empty(),
            "{}: every puzzle needs an accepted action",
            puzzle.name
        );
    }
}
