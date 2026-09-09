//! Value-guided information-set Monte Carlo tree search.
//!
//! The older `MctsPlayer` (player code `mr`) samples uniformly random playouts to the end of the
//! game from a tree of concrete `State`s. That is unusable in a hidden-information game: the tree
//! is built over states that embed the opponent's actual hand, and a random playout is a very
//! noisy estimator of a position.
//!
//! This player replaces both halves:
//!
//! * **Information sets, not states.** Every iteration re-deals the opponent's hand from the cards
//!   they have not shown (their hand plus their remaining deck list, which the simulator knows) and
//!   re-shuffles our own remaining deck, then replays the tree from the root on that
//!   determinisation. Tree edges are keyed by `Action`, not by `State`, so the same edge covers
//!   every determinisation in which that action was legal. Selection uses the information-set UCB
//!   of Cowling, Powley and Whitehouse: the exploration term counts how often an action was
//!   *available*, not how often the parent was visited, so actions that are only legal in a few
//!   determinisations are not punished for it.
//! * **A value function, not a playout to the end.** The search has a horizon of two further turns
//!   past the one being played (our turn, the opponent's reply, our next turn). Leaves are scored
//!   with the injected `ValueFunction` trait object, squashed into [-1, 1], so the numbers UCB sees
//!   are bounded. Inside the horizon, untried actions are explored with a depth-2 expectiminimax
//!   greedy policy rather than at random.
//!
//! Chance (coin flips, draws, shuffles) is sampled: `apply_action` resolves a branch with the
//! search RNG, so a chance node is visited under a different outcome on different iterations and
//! the visit counts average over them.

use log::{trace, LevelFilter};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::fmt::Debug;

use super::{value_functions::baseline_value_function, Player, ValueFunction};
use crate::{
    actions::{apply_action, forecast_action, Action},
    models::Card,
    state::GameOutcome,
    Deck, State,
};

/// Iterations used by the bare player code `m`.
pub const DEFAULT_ITERATIONS: u64 = 200;

/// Turns simulated past the one being played before a leaf is scored. Two extra turns means our
/// turn, the opponent's reply and our next turn.
pub const DEFAULT_HORIZON_TURNS: u8 = 2;

/// Depth of the expectiminimax greedy policy that picks which untried action a node explores
/// first. Depth 2 is the `e2` policy: score every action by the best follow-up action it allows.
const EXPANSION_POLICY_DEPTH: usize = 2;

/// Depth of the greedy policy driving the playout from a freshly expanded node to the horizon.
///
/// Depth 2 here as well is the natural reading of the plan, and it is what this player did first,
/// but it costs a squared number of state clones at *every* step of a playout that runs to the end
/// of two turns: measured on this machine a single 200-iteration decision took 6.4 s, three times
/// the 2 s budget. Depth 1 (score each action by the position it leaves, expectation taken over
/// its chance branches) keeps the frontier choice fully informed at depth 2 while bringing a
/// decision back inside the budget.
const PLAYOUT_POLICY_DEPTH: usize = 1;

/// UCB exploration constant. Rewards live in [-1, 1], so this is the usual sqrt(2)-ish range.
const EXPLORATION: f64 = 1.0;

/// Value-function output that maps to a reward of tanh(1) = 0.76. One point of the three needed to
/// win is worth 10,000 to the baseline function, so a whole point moves the reward by about 0.46
/// and the positional terms (hundreds) still separate otherwise equal lines.
const REWARD_SCALE: f64 = 20_000.0;

/// Progressive widening: a node with `n` visits may hold at most `C * n^ALPHA` children. Only
/// applied once a node has more legal actions than `WIDENING_MIN_ACTIONS`, so small decisions are
/// still enumerated exhaustively.
const WIDENING_C: f64 = 2.0;
const WIDENING_ALPHA: f64 = 0.5;
const WIDENING_MIN_ACTIONS: usize = 8;

/// Hard stop on a single playout, so a pathological loop of stack actions cannot hang a decision.
const MAX_PLAYOUT_ACTIONS: usize = 300;

/// The two constants that every node of one search shares: whose decision it is, and the turn the
/// search started on (from which the horizon is measured). Bundled so the recursive functions stay
/// inside clippy's argument budget.
#[derive(Debug, Clone, Copy)]
struct SearchFrame {
    myself: usize,
    start_turn: u8,
}

pub struct MctsInformedPlayer {
    pub deck: Deck,
    pub iterations: u64,
    pub horizon_turns: u8,
    pub value_function: ValueFunction,
}

impl MctsInformedPlayer {
    /// Builds the player with the baseline hand-tuned value function.
    pub fn new(deck: Deck, iterations: u64) -> Self {
        Self::with_value_function(deck, iterations, Box::new(baseline_value_function))
    }

    /// Builds the player with an injected value function, so a learned one can be dropped in
    /// without touching the search.
    pub fn with_value_function(deck: Deck, iterations: u64, value_function: ValueFunction) -> Self {
        Self {
            deck,
            iterations,
            horizon_turns: DEFAULT_HORIZON_TURNS,
            value_function,
        }
    }
}

impl Player for MctsInformedPlayer {
    fn decision_fn(
        &mut self,
        rng: &mut StdRng,
        state: &State,
        possible_actions: &[Action],
    ) -> Action {
        let myself = possible_actions[0].actor;
        if possible_actions.len() == 1 {
            return possible_actions[0].clone();
        }

        // The search RNG is seeded from one draw off the game RNG, so a decision is a pure
        // function of the game seed and the actions played so far. It consumes exactly one u64
        // per decision whatever the iteration count, which keeps games reproducible.
        let search_seed: u64 = rng.gen();
        let mut search_rng = StdRng::seed_from_u64(search_seed);

        // Silence the engine's per-action logging for the duration of the search, the way
        // ExpectiMiniMaxPlayer does: a 200-iteration search applies tens of thousands of actions.
        let original_level = log::max_level();
        log::set_max_level(LevelFilter::Info);

        let frame = SearchFrame {
            myself,
            start_turn: state.turn_count,
        };
        let mut root = Node::default();
        for _ in 0..self.iterations {
            let mut sim_state = state.clone();
            determinize(&mut sim_state, myself, &mut search_rng);
            self.visit(&mut root, &mut sim_state, &mut search_rng, frame);
        }

        log::set_max_level(original_level);

        // Most visited root child, ties broken by mean reward. The tie-break is not cosmetic: at
        // low iteration counts most children have exactly one visit, and without it the choice
        // falls to whichever child happens to sit last in the vector, which is the *worst* action
        // by the greedy expansion order.
        let chosen = root
            .children
            .iter()
            .filter(|edge| possible_actions.contains(&edge.action))
            .max_by(|a, b| {
                let key =
                    |edge: &Edge| (edge.node.visits, edge.node.total_reward / edge.node.visits);
                let (a_visits, a_mean) = key(a);
                let (b_visits, b_mean) = key(b);
                a_visits
                    .partial_cmp(&b_visits)
                    .expect("visit counts are finite")
                    .then(
                        a_mean
                            .partial_cmp(&b_mean)
                            .expect("mean rewards are finite"),
                    )
            })
            .map(|edge| edge.action.clone());

        trace!(
            "MctsInformed({myself}) seed {search_seed} chose {chosen:?} from {} root children",
            root.children.len()
        );

        // The determinisation never touches our own board or hand, so the root's legal actions are
        // the ones we were handed. The fallback is defensive only.
        chosen.unwrap_or_else(|| possible_actions[0].clone())
    }

    fn get_deck(&self) -> Deck {
        self.deck.clone()
    }
}

impl MctsInformedPlayer {
    /// One iteration of selection, expansion, evaluation and backpropagation. `state` is mutated
    /// down the path being walked. Returns the reward from `myself`'s point of view.
    fn visit(
        &self,
        node: &mut Node,
        state: &mut State,
        rng: &mut StdRng,
        frame: SearchFrame,
    ) -> f64 {
        let reward = if self.past_horizon(state, frame.start_turn) {
            self.evaluate(state, frame.myself)
        } else {
            let (actor, actions) = state.generate_possible_actions();
            if actions.is_empty() {
                self.evaluate(state, frame.myself)
            } else {
                self.visit_actions(node, state, rng, frame, actor, &actions)
            }
        };
        node.visits += 1.0;
        node.total_reward += reward;
        reward
    }

    fn visit_actions(
        &self,
        node: &mut Node,
        state: &mut State,
        rng: &mut StdRng,
        frame: SearchFrame,
        actor: usize,
        actions: &[Action],
    ) -> f64 {
        // Every action legal in this determinisation that already has an edge was "available", so
        // its exploration bonus grows even on iterations where a different action was taken.
        let mut legal_children: Vec<usize> = Vec::with_capacity(node.children.len());
        for (index, edge) in node.children.iter_mut().enumerate() {
            if actions.contains(&edge.action) {
                edge.availability += 1.0;
                legal_children.push(index);
            }
        }

        let untried: Vec<Action> = actions
            .iter()
            .filter(|action| !node.children.iter().any(|edge| edge.action == **action))
            .cloned()
            .collect();

        let cap = widening_cap(node.visits, actions.len());
        let must_expand = legal_children.is_empty();
        if !untried.is_empty() && (node.children.len() < cap || must_expand) {
            let action = self
                .greedy_action(rng, state, &untried, actor, EXPANSION_POLICY_DEPTH)
                .clone();
            apply_action(rng, state, &action);
            let reward = self.playout(state, rng, frame);
            node.children.push(Edge {
                action,
                availability: 1.0,
                node: Node {
                    visits: 1.0,
                    total_reward: reward,
                    children: Vec::new(),
                },
            });
            return reward;
        }

        let index = self.select_child(node, &legal_children, actor == frame.myself);
        let action = node.children[index].action.clone();
        apply_action(rng, state, &action);
        self.visit(&mut node.children[index].node, state, rng, frame)
    }

    /// Information-set UCB. `maximizing` is false on the opponent's nodes, where the same reward
    /// scale is read upside down, so the search assumes the opponent plays to beat us.
    fn select_child(&self, node: &Node, legal_children: &[usize], maximizing: bool) -> usize {
        let mut best_index = legal_children[0];
        let mut best_score = f64::NEG_INFINITY;
        for &index in legal_children {
            let edge = &node.children[index];
            let mean = edge.node.total_reward / edge.node.visits;
            let exploit = if maximizing { mean } else { -mean };
            let explore = EXPLORATION * (edge.availability.ln() / edge.node.visits).sqrt();
            let score = exploit + explore;
            if score > best_score {
                best_score = score;
                best_index = index;
            }
        }
        best_index
    }

    /// Plays on from a freshly expanded node with the greedy policy until the horizon, then scores
    /// the position. This is the "informed" half of the rollout: no random play at any point.
    fn playout(&self, state: &mut State, rng: &mut StdRng, frame: SearchFrame) -> f64 {
        let mut steps = 0;
        while !self.past_horizon(state, frame.start_turn) && steps < MAX_PLAYOUT_ACTIONS {
            let (actor, actions) = state.generate_possible_actions();
            if actions.is_empty() {
                break;
            }
            let action = if actions.len() == 1 {
                actions[0].clone()
            } else {
                self.greedy_action(rng, state, &actions, actor, PLAYOUT_POLICY_DEPTH)
                    .clone()
            };
            apply_action(rng, state, &action);
            steps += 1;
        }
        self.evaluate(state, frame.myself)
    }

    /// The expectiminimax greedy policy at `depth`: score each action by the expectation over its
    /// chance branches of the best line `actor` still has within the depth, then take the best.
    /// `depth` 2 is the `e2` policy, `depth` 1 scores the position each action leaves behind.
    fn greedy_action<'a>(
        &self,
        rng: &mut StdRng,
        state: &State,
        actions: &'a [Action],
        actor: usize,
        depth: usize,
    ) -> &'a Action {
        let mut best_index = 0;
        let mut best_score = f64::NEG_INFINITY;
        for (index, action) in actions.iter().enumerate() {
            let score = self.action_score(rng, state, action, actor, depth - 1);
            if score > best_score {
                best_score = score;
                best_index = index;
            }
        }
        &actions[best_index]
    }

    /// Expectation of `state_score` over the chance branches of one action.
    fn action_score(
        &self,
        rng: &mut StdRng,
        state: &State,
        action: &Action,
        actor: usize,
        depth: usize,
    ) -> f64 {
        let (probabilities, mutations) = forecast_action(state, action).into_branches();
        let mut score = 0.0;
        for (probability, mutation) in probabilities.iter().zip(mutations) {
            let mut outcome = state.clone();
            mutation(rng, &mut outcome, action);
            score += probability * self.state_score(rng, &outcome, actor, depth);
        }
        score
    }

    fn state_score(&self, rng: &mut StdRng, state: &State, actor: usize, depth: usize) -> f64 {
        if depth == 0 || state.winner.is_some() || state.current_player != actor {
            return (self.value_function)(state, actor);
        }
        let (next_actor, actions) = state.generate_possible_actions();
        if next_actor != actor || actions.is_empty() {
            return (self.value_function)(state, actor);
        }
        actions
            .iter()
            .map(|action| self.action_score(rng, state, action, actor, depth - 1))
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// True once the game is over or the horizon has been passed. `turn_count` advances by one per
    /// player turn, so `start_turn + horizon_turns + 1` is the start of the turn after our next.
    fn past_horizon(&self, state: &State, start_turn: u8) -> bool {
        state.winner.is_some()
            || state.turn_count
                >= start_turn
                    .saturating_add(self.horizon_turns)
                    .saturating_add(1)
    }

    /// Leaf score in [-1, 1]. A finished game is scored exactly; anything else is the value
    /// function squashed, which keeps the UCB exploration term meaningful.
    fn evaluate(&self, state: &State, myself: usize) -> f64 {
        match state.winner {
            Some(GameOutcome::Win(winner)) => {
                if winner == myself {
                    1.0
                } else {
                    -1.0
                }
            }
            Some(GameOutcome::Tie) => 0.0,
            None => ((self.value_function)(state, myself) / REWARD_SCALE).tanh(),
        }
    }
}

impl Debug for MctsInformedPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MctsInformedPlayer with {} iterations, horizon {} turns",
            self.iterations, self.horizon_turns
        )
    }
}

/// Samples one consistent world: the opponent's hand is re-dealt from everything they have not
/// played (hand plus remaining deck), and our own remaining deck is re-shuffled so the search does
/// not plan around draws it cannot actually see. Public information (both boards, both discard
/// piles, hand *sizes*, the deck lists) is untouched.
///
/// Skipped on turn 0, where the setup rules require each opening hand to contain a Basic and a
/// re-deal could produce a hand with no legal placement.
fn determinize(state: &mut State, myself: usize, rng: &mut StdRng) {
    state.decks[myself].cards.shuffle(rng);

    if state.turn_count == 0 {
        return;
    }
    let opponent = (myself + 1) % 2;
    let hand_size = state.hands[opponent].len();
    if hand_size == 0 {
        return;
    }

    let mut pool: Vec<Card> = Vec::with_capacity(hand_size + state.decks[opponent].cards.len());
    pool.extend(state.hands[opponent].iter().cloned());
    pool.extend(state.decks[opponent].cards.iter().cloned());
    pool.shuffle(rng);

    state.decks[opponent].cards = pool.split_off(hand_size);
    state.hands[opponent] = pool;
}

/// How many children a node with `visits` visits is allowed. Decisions with few actions are
/// enumerated in full; only wide ones (a hand full of playable Trainers, an Attach with many
/// targets) are widened gradually.
fn widening_cap(visits: f64, action_count: usize) -> usize {
    if action_count <= WIDENING_MIN_ACTIONS {
        return action_count;
    }
    let cap = (WIDENING_C * (visits + 1.0).powf(WIDENING_ALPHA)).ceil() as usize;
    cap.clamp(1, action_count)
}

#[derive(Default)]
struct Node {
    visits: f64,
    total_reward: f64,
    children: Vec<Edge>,
}

struct Edge {
    action: Action,
    /// Number of iterations in which this action was legal, used by the information-set UCB.
    availability: f64,
    node: Node,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::load_test_decks;

    #[test]
    fn test_widening_cap_enumerates_small_action_sets() {
        assert_eq!(widening_cap(0.0, 4), 4);
        assert_eq!(
            widening_cap(200.0, WIDENING_MIN_ACTIONS),
            WIDENING_MIN_ACTIONS
        );
    }

    #[test]
    fn test_widening_cap_grows_with_visits() {
        assert!(widening_cap(0.0, 40) < widening_cap(200.0, 40));
        assert!(widening_cap(0.0, 40) >= 1);
        assert_eq!(widening_cap(1_000_000.0, 40), 40);
    }

    #[test]
    fn test_determinize_keeps_public_information() {
        let (deck_a, deck_b) = load_test_decks();
        let mut state = State::initialize(&deck_a, &deck_b, &mut rand::thread_rng());
        state.turn_count = 5;

        let before_hand_size = state.hands[1].len();
        let before_deck_size = state.decks[1].cards.len();
        let mut pool_before: Vec<String> = state.hands[1]
            .iter()
            .chain(state.decks[1].cards.iter())
            .map(|card| card.get_name())
            .collect();
        pool_before.sort();

        determinize(&mut state, 0, &mut StdRng::seed_from_u64(7));

        assert_eq!(state.hands[1].len(), before_hand_size);
        assert_eq!(state.decks[1].cards.len(), before_deck_size);
        let mut pool_after: Vec<String> = state.hands[1]
            .iter()
            .chain(state.decks[1].cards.iter())
            .map(|card| card.get_name())
            .collect();
        pool_after.sort();
        assert_eq!(pool_before, pool_after);
    }

    #[test]
    fn test_determinize_leaves_setup_hands_alone() {
        let (deck_a, deck_b) = load_test_decks();
        let mut state = State::initialize(&deck_a, &deck_b, &mut rand::thread_rng());
        assert_eq!(state.turn_count, 0);

        let before = state.hands[1].clone();
        determinize(&mut state, 0, &mut StdRng::seed_from_u64(7));
        assert_eq!(state.hands[1], before);
    }
}
