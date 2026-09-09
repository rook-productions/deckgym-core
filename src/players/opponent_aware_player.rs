//! A player that searches its own turn like `ExpectiMiniMaxPlayer` does, and then, instead of
//! scoring the end-of-turn position straight away, samples the opponent's hidden hand, lets the
//! opponent take their whole reply turn under a fast policy, and scores what is left.
//!
//! Why it is not simply "search two plies": the simulator holds perfect information, so a search
//! that walked the opponent's real actions would be reading their hand. This player is only
//! allowed to know what a human at the table knows plus one concession the engine forces on us:
//! the multiset of cards the opponent has not yet revealed (their remaining deck together with
//! their hand). It never looks at *which* of those cards are in hand. Every reply is played from a
//! determinisation: the unseen multiset is reshuffled and dealt into a hand of the right size.
//!
//! Cost control, in the order it bites:
//!   1. duplicate own-turn actions are pruned before the search starts,
//!   2. identical end-of-turn positions are memoised for the duration of one decision,
//!   3. a node budget counts every action applied inside a reply rollout, and once it is spent the
//!      remaining leaves are scored with the plain value function, which degrades this player back
//!      into `ExpectiMiniMaxPlayer` rather than blowing the turn clock. That fallback is logged.

use log::{debug, trace, warn};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use crate::actions::{apply_action, forecast_action, Action};
use crate::models::Card;
use crate::state::PlayedCard;
use crate::{Deck, State};

use super::expectiminimax_player::{argmax_score, search_actions, ValueFnRef};
use super::{Player, ValueFunction};

/// Plies of own-turn search for the bare `o` code.
pub const DEFAULT_DEPTH: usize = 3;
/// Determinisations of the opponent's hand per end-of-turn position for the bare `o` code.
pub const DEFAULT_SAMPLES: usize = 3;
/// Actions that may be applied inside reply rollouts during a single decision.
///
/// Measured over 1,146 depth-3 decisions in 15 `o` vs `o` games on venusaur-exeggutor against
/// weezing-arbok: mean 186 nodes per decision, median 134, 90th percentile 418, 99th percentile
/// 744, largest 1,437. This budget leaves that distribution alone with room to spare and still
/// stops a pathological board from spending an unbounded turn. The fallback did not fire once in
/// 50 games of `o` vs `o` at this setting.
pub const DEFAULT_NODE_BUDGET: usize = 8_000;
/// Actions a single reply rollout may apply before it is cut short. A real turn is well under
/// this; the cap only exists so a rollout cannot loop forever on a board with a repeating action.
pub const DEFAULT_REPLY_STEP_CAP: usize = 80;
/// Depth of the search the opponent uses for their reply.
pub const DEFAULT_REPLY_DEPTH: usize = 2;

/// How the opponent plays their sampled reply turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyPolicy {
    /// One ply: score each legal action's expected successor with the value function and take the
    /// best. Cheapest, and equivalent to `ExpectiMiniMax { depth: 1 }`.
    Greedy,
    /// The `e` search to the given depth, from the replying player's own perspective.
    ExpectiMiniMax { depth: usize },
}

impl ReplyPolicy {
    fn depth(self) -> usize {
        match self {
            ReplyPolicy::Greedy => 1,
            ReplyPolicy::ExpectiMiniMax { depth } => depth.max(1),
        }
    }
}

pub struct OpponentAwarePlayer {
    pub deck: Deck,
    /// Plies of own-turn search, same meaning as `ExpectiMiniMaxPlayer::max_depth`.
    pub max_depth: usize,
    /// Determinisations of the opponent's hidden hand per end-of-turn position (K).
    pub samples: usize,
    pub reply_policy: ReplyPolicy,
    pub node_budget: usize,
    pub reply_step_cap: usize,
    /// Scores a position from a player's perspective. A trait object so the learned value function
    /// can be dropped in without touching this file.
    pub value_function: ValueFunction,
    /// Private randomness for determinisation and rollouts, reseeded from the game's rng at every
    /// decision so samples differ between games without disturbing the search's own draws.
    rng: RefCell<StdRng>,
    /// Decisions so far in which the node budget ran out and leaves fell back to the plain value
    /// function.
    fallbacks: Cell<usize>,
    /// Decisions made so far.
    decisions: Cell<usize>,
}

impl OpponentAwarePlayer {
    pub fn new(deck: Deck, max_depth: usize, samples: usize) -> Self {
        Self {
            deck,
            max_depth: max_depth.max(1),
            samples: samples.max(1),
            reply_policy: ReplyPolicy::ExpectiMiniMax {
                depth: DEFAULT_REPLY_DEPTH,
            },
            node_budget: DEFAULT_NODE_BUDGET,
            reply_step_cap: DEFAULT_REPLY_STEP_CAP,
            value_function: Box::new(super::value_functions::baseline_value_function),
            rng: RefCell::new(StdRng::seed_from_u64(0)),
            fallbacks: Cell::new(0),
            decisions: Cell::new(0),
        }
    }

    /// How many decisions exhausted the node budget and fell back to plain expectiminimax.
    pub fn fallback_count(&self) -> usize {
        self.fallbacks.get()
    }

    /// How many decisions this player has made.
    pub fn decision_count(&self) -> usize {
        self.decisions.get()
    }
}

impl Player for OpponentAwarePlayer {
    fn decision_fn(
        &mut self,
        rng: &mut StdRng,
        state: &State,
        possible_actions: &[Action],
    ) -> Action {
        let myself = possible_actions[0].actor;
        self.decisions.set(self.decisions.get() + 1);

        // Reseed the private rng from the game's rng so that determinisations vary between games
        // (and between the two seats) instead of replaying one fixed sample sequence.
        let seed: u64 = rng.gen();
        self.rng.replace(StdRng::seed_from_u64(seed));

        let candidates = {
            let mut own_rng = self.rng.borrow_mut();
            prune_duplicate_actions(&mut own_rng, state, possible_actions)
        };
        if candidates.len() < possible_actions.len() {
            trace!(
                "OpponentAware pruned {} of {} own actions",
                possible_actions.len() - candidates.len(),
                possible_actions.len()
            );
        }

        let nodes_used = Cell::new(0usize);
        let budget_spent = Cell::new(false);
        let memo: RefCell<HashMap<u64, f64>> = RefCell::new(HashMap::new());

        // Borrow the pieces the leaf evaluator needs. All of these are shared borrows of `self`,
        // so they coexist, and none of them touch the game's `rng`.
        let base: ValueFnRef = &*self.value_function;
        let own_rng = &self.rng;
        let samples = self.samples;
        let policy = self.reply_policy;
        let step_cap = self.reply_step_cap;
        let budget = self.node_budget;

        let leaf = |leaf_state: &State, actor: usize| -> f64 {
            opponent_reply_value(
                leaf_state,
                actor,
                base,
                &mut own_rng.borrow_mut(),
                samples,
                policy,
                step_cap,
                budget,
                &nodes_used,
                &budget_spent,
                &memo,
            )
        };

        let (scores, _nodes) =
            search_actions(rng, state, &candidates, self.max_depth, myself, &leaf);
        let (best_idx, best_score) = argmax_score(&scores);

        if budget_spent.get() {
            self.fallbacks.set(self.fallbacks.get() + 1);
            warn!(
                "OpponentAware node budget of {} exhausted on turn {} for player {}; \
                 remaining leaves fell back to plain expectiminimax",
                budget, state.turn_count, myself
            );
        }
        debug!(
            "OpponentAware chose action {best_idx} with score {best_score} \
             ({} reply nodes, {} memoised positions)",
            nodes_used.get(),
            memo.borrow().len()
        );

        candidates[best_idx].clone()
    }

    fn get_deck(&self) -> Deck {
        self.deck.clone()
    }
}

impl Debug for OpponentAwarePlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OpponentAwarePlayer(depth {}, K {})",
            self.max_depth, self.samples
        )
    }
}

/// Value of an end-of-turn position: the average, over `samples` determinisations of the
/// opponent's hidden hand, of the position left once the opponent has taken their reply turn.
///
/// Falls through to the plain value function when there is no reply to search (the game is over,
/// or the search stopped inside our own turn) and when the node budget has run out.
#[allow(clippy::too_many_arguments)]
fn opponent_reply_value(
    leaf: &State,
    myself: usize,
    base: ValueFnRef,
    rng: &mut StdRng,
    samples: usize,
    policy: ReplyPolicy,
    step_cap: usize,
    budget: usize,
    nodes_used: &Cell<usize>,
    budget_spent: &Cell<bool>,
    memo: &RefCell<HashMap<u64, f64>>,
) -> f64 {
    // Nothing to reply to: either the game ended inside our turn, or the depth ran out while we
    // still had the initiative. Score as `e` would.
    if leaf.is_game_over() || leaf.current_player == myself {
        return base(leaf, myself);
    }

    // The memo comes before the budget check: a position that has already been played out costs
    // nothing to reuse, so there is no reason to degrade it to the plain value function.
    let key = position_key(leaf);
    if let Some(cached) = memo.borrow().get(&key) {
        return *cached;
    }

    if nodes_used.get() >= budget {
        budget_spent.set(true);
        return base(leaf, myself);
    }

    let mut total = 0.0;
    for _ in 0..samples {
        let mut sampled = determinise_opponent_hand(leaf, myself, rng);
        play_out_reply(&mut sampled, rng, base, policy, step_cap, nodes_used);
        total += base(&sampled, myself);
    }
    let value = total / samples as f64;
    memo.borrow_mut().insert(key, value);
    value
}

/// Reshuffles everything the opponent has not revealed and deals them a fresh hand of the same
/// size. Their in-play Pokémon, attached energy, discard pile, points and Energy Zone are public
/// and are left exactly as they are; only the hand/deck split is resampled.
fn determinise_opponent_hand(state: &State, myself: usize, rng: &mut StdRng) -> State {
    let opponent = (myself + 1) % 2;
    let mut sampled = state.clone();

    let hand_size = sampled.hands[opponent].len();
    let mut unseen: Vec<Card> = sampled.decks[opponent].cards.clone();
    unseen.extend(sampled.hands[opponent].iter().cloned());
    unseen.shuffle(rng);

    let split_at = hand_size.min(unseen.len());
    let remaining_deck = unseen.split_off(split_at);
    sampled.hands[opponent] = unseen;
    sampled.decks[opponent].cards = remaining_deck;
    sampled
}

/// Plays out the rest of the turn currently in progress. The turn is over when `turn_count` moves
/// on, which only `advance_turn` does, so this is robust to mid-turn hand-offs (a promotion forced
/// by Sabrina, say) that flip `current_player` without ending anybody's turn.
///
/// At a normal end-of-turn leaf that means the opponent's whole reply turn, starting from the
/// Energy Zone rotation and the draw that `advance_turn` has already queued, which is why the
/// determinisation has to happen before this runs: the draw comes off the resampled deck. At the
/// rarer leaf where control passed to the opponent for a forced decision inside our own turn, it
/// means the remainder of our own turn instead, and the position is scored there. Each action is
/// chosen by the reply policy from the acting player's own perspective, so both cases are played
/// by someone trying to win.
fn play_out_reply(
    state: &mut State,
    rng: &mut StdRng,
    base: ValueFnRef,
    policy: ReplyPolicy,
    step_cap: usize,
    nodes_used: &Cell<usize>,
) {
    let start_turn = state.turn_count;
    let mut steps = 0;
    while !state.is_game_over() && state.turn_count == start_turn && steps < step_cap {
        let (actor, actions) = state.generate_possible_actions();
        let Some(first) = actions.first() else {
            break;
        };
        let action = if actions.len() == 1 {
            first.clone()
        } else {
            let (scores, _) = search_actions(rng, state, &actions, policy.depth(), actor, base);
            let (idx, _) = argmax_score(&scores);
            actions[idx].clone()
        };
        apply_action(rng, state, &action);
        steps += 1;
        nodes_used.set(nodes_used.get() + 1);
    }
}

/// Drops own-turn actions that lead to a position already reachable by an earlier action, which is
/// the honest version of "prune dominated actions": two lines with the same successor have the
/// same value, so searching the second one is pure waste. Bench slots are order-insensitive in
/// this game, so the key sorts them, which is what catches attaching the same Energy to two
/// identical Bench Pokémon.
///
/// Only single-outcome actions are compared. Anything that branches (a coin flip, a draw with
/// several possible results) is kept as-is rather than judged on one sampled outcome.
fn prune_duplicate_actions(rng: &mut StdRng, state: &State, actions: &[Action]) -> Vec<Action> {
    if actions.len() < 3 {
        return actions.to_vec();
    }
    let mut seen: HashSet<u64> = HashSet::with_capacity(actions.len());
    let mut kept: Vec<Action> = Vec::with_capacity(actions.len());
    for action in actions {
        let (probabilities, mutations) = forecast_action(state, action).into_branches();
        if probabilities.len() != 1 {
            kept.push(action.clone());
            continue;
        }
        let mut successor = state.clone();
        for mutation in mutations {
            mutation(rng, &mut successor, action);
        }
        if seen.insert(position_key(&successor)) {
            kept.push(action.clone());
        }
    }
    kept
}

/// A hash that treats Bench orderings as the same position.
///
/// The Bench is a set as far as the rules go, but `State` stores it as four indexed slots, so two
/// otherwise identical positions hash differently when the same Pokémon sit in a different order.
/// Sorting the slots fixes that. The one case where slot indices carry meaning is a pending
/// decision on the move-generation stack, which names Pokémon by index; those positions keep the
/// plain hash.
fn position_key(state: &State) -> u64 {
    let mut hasher = DefaultHasher::new();
    if state.move_generation_stack.is_empty() {
        let mut canonical = state.clone();
        for player in 0..2 {
            let mut bench: Vec<Option<PlayedCard>> =
                canonical.in_play_pokemon[player][1..].to_vec();
            bench.sort_by_key(hash_slot);
            for (offset, slot) in bench.into_iter().enumerate() {
                canonical.in_play_pokemon[player][1 + offset] = slot;
            }
        }
        canonical.hash(&mut hasher);
    } else {
        state.hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_slot(slot: &Option<PlayedCard>) -> u64 {
    let mut hasher = DefaultHasher::new();
    slot.hash(&mut hasher);
    hasher.finish()
}

/// Test hook for the determinisation. `tests/bots/` needs to check that the opponent's hand is
/// resampled from the unseen multiset rather than read off the state, and that is a property of
/// this private function rather than of any decision the player makes.
#[cfg(feature = "test-utils")]
pub fn determinise_opponent_hand_for_test(state: &State, myself: usize, rng: &mut StdRng) -> State {
    determinise_opponent_hand(state, myself, rng)
}
