//! `--data-output` exports a per-game `result.json` alongside the per-ply files.
//!
//! Driven through the public `Simulation` API (the same path `deckgym simulate --data-output`
//! takes) so the test covers the wiring as well as the exporter itself.

use deckgym::data_exporter::{DataExporter, GameResult};
use deckgym::players::{parse_player_code, PlayerCode};
use deckgym::state::GameOutcome;
use deckgym::{Deck, Simulation};
use std::path::PathBuf;

/// A scratch folder unique to this test, removed and recreated on every run.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("deckgym_data_export_test_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("failed to create scratch dir");
    dir
}

fn player_codes(code: &str) -> Vec<PlayerCode> {
    vec![
        parse_player_code(code).expect("valid player code"),
        parse_player_code(code).expect("valid player code"),
    ]
}

#[test]
fn test_data_output_writes_a_result_json_per_game() {
    let deck_a_path = "example_decks/venusaur-exeggutor.txt";
    let deck_b_path = "example_decks/weezing-arbok.txt";
    let out_dir = scratch_dir("two_games");

    let deck_a = Deck::from_file(deck_a_path).expect("deck a parses");
    let deck_b = Deck::from_file(deck_b_path).expect("deck b parses");

    let out_path = out_dir.clone();
    let mut simulation =
        Simulation::new_with_decks(deck_a, deck_b, player_codes("e"), 2, None, false, Some(1))
            .expect("simulation builds")
            .register_with_closure(move || {
                Box::new(DataExporter::new(out_path.clone()).with_decks(deck_a_path, deck_b_path))
            });

    let outcomes = simulation.run();
    assert_eq!(outcomes.len(), 2, "asked for two games");

    // One folder per game, each with ply files and exactly one result.json.
    let mut game_dirs: Vec<PathBuf> = std::fs::read_dir(&out_dir)
        .expect("output dir readable")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.is_dir())
        .collect();
    game_dirs.sort();
    assert_eq!(game_dirs.len(), 2, "one folder per game in {out_dir:?}");

    let mut results = vec![];
    for game_dir in &game_dirs {
        let ply_count = std::fs::read_dir(game_dir)
            .expect("game dir readable")
            .filter(|entry| {
                entry
                    .as_ref()
                    .map(|e| e.file_name().to_string_lossy().starts_with("ply_"))
                    .unwrap_or(false)
            })
            .count();
        assert!(ply_count > 0, "game {game_dir:?} exported no ply files");

        let raw = std::fs::read_to_string(game_dir.join("result.json"))
            .unwrap_or_else(|e| panic!("no result.json in {game_dir:?}: {e}"));
        let result: GameResult = serde_json::from_str(&raw).expect("result.json parses");

        assert_eq!(result.deck_a.as_deref(), Some(deck_a_path));
        assert_eq!(result.deck_b.as_deref(), Some(deck_b_path));
        assert!(
            matches!(result.starting_player, Some(0) | Some(1)),
            "starting_player should be 0 or 1, got {:?}",
            result.starting_player
        );
        assert!(
            result.turns > 0 && result.turns <= 31,
            "turns out of range: {}",
            result.turns
        );
        results.push(result);
    }

    // The recorded winner must agree with the outcome the simulation returned, and with the
    // points in the same file: a winner has 3 points unless the loser ran out of Pokemon.
    let mut by_winner = results.iter().map(|r| r.winner).collect::<Vec<_>>();
    let mut expected = outcomes
        .iter()
        .map(|outcome| match outcome {
            Some(GameOutcome::Win(player)) => Some(*player),
            Some(GameOutcome::Tie) | None => None,
        })
        .collect::<Vec<_>>();
    by_winner.sort();
    expected.sort();
    assert_eq!(
        by_winner, expected,
        "result.json winners disagree with the returned outcomes"
    );

    for result in &results {
        match result.winner {
            Some(winner) => {
                let loser = 1 - winner;
                assert!(
                    result.points[winner] >= result.points[loser],
                    "winner {winner} has fewer points than the loser in {result:?}"
                );
            }
            None => {
                assert!(
                    result.points[0] < 3 && result.points[1] < 3,
                    "a tie should not have a player on 3 points: {result:?}"
                );
            }
        }
    }

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn test_data_output_without_deck_labels_exports_nulls() {
    let out_dir = scratch_dir("no_labels");
    let deck_a = Deck::from_file("example_decks/venusaur-exeggutor.txt").expect("deck a parses");
    let deck_b = Deck::from_file("example_decks/weezing-arbok.txt").expect("deck b parses");

    let out_path = out_dir.clone();
    let mut simulation =
        Simulation::new_with_decks(deck_a, deck_b, player_codes("e2"), 1, None, false, Some(1))
            .expect("simulation builds")
            .register_with_closure(move || Box::new(DataExporter::new(out_path.clone())));
    simulation.run();

    let game_dir = std::fs::read_dir(&out_dir)
        .expect("output dir readable")
        .map(|entry| entry.expect("readable entry").path())
        .find(|path| path.is_dir())
        .expect("one game folder");
    let raw = std::fs::read_to_string(game_dir.join("result.json")).expect("result.json exists");
    let result: GameResult = serde_json::from_str(&raw).expect("result.json parses");
    assert_eq!(result.deck_a, None);
    assert_eq!(result.deck_b, None);

    let _ = std::fs::remove_dir_all(&out_dir);
}
