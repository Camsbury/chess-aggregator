use crate::GameStats;
use radix_trie::{Trie, SubTrie, TrieCommon};
use shakmaty::Chess;

type StatsTree = Trie<String, GameStats>;
type StatsST<'a> = SubTrie<'a, String, GameStats>;

struct TraversalStep<'a> {
    tree: StatsST<'a>,
    game_stack: Vec<Chess>,
}

impl TraversalStep<'_> {
    fn new(tree: &StatsTree) -> TraversalStep {
        Self::from_subtree(
            tree.children().next().unwrap(),
            vec![Chess::new()],
        )
    }

    fn from_subtree(subtree: StatsST, game_stack: Vec<Chess>) -> TraversalStep {
        TraversalStep {
            tree: subtree,
            game_stack,
        }
    }
}

pub fn extract_stats(tree: Trie<String, GameStats>) {
    let mut stack = vec![TraversalStep::new(&tree)];
    while !stack.is_empty() {
        let step = stack.pop().unwrap();
        for child in step.tree.children() {
            stack.push(TraversalStep::from_subtree(
                child,
                vec![Chess::new()],
            ));
        }
    }
}
