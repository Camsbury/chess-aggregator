use crate::GameStats;
use nibble_vec::Nibblet;
use radix_trie::{Trie, SubTrie, TrieCommon};
use shakmaty::Chess;

const SEPARATOR: u8 = 32;

type StatsTree = Trie<String, GameStats>;
type StatsST<'a> = SubTrie<'a, String, GameStats>;

struct TraversalStep<'a> {
    tree: StatsST<'a>,
    game_stack: Vec<Chess>,
    prefix_acc: Nibblet,
    offset: usize,
}

impl TraversalStep<'_> {
    fn new(tree: &StatsTree) -> TraversalStep {
        Self::build_step(
            tree.children().next().unwrap(),
            vec![Chess::new()],
            Nibblet::new(),
            1,
        )
    }

    fn build_step(
        tree: StatsST,
        game_stack: Vec<Chess>,
        prefix_acc: Nibblet,
        offset: usize,
    ) -> TraversalStep {
        TraversalStep {
            tree,
            game_stack,
            prefix_acc,
            offset,
        }
    }
}

pub fn extract_stats(tree: Trie<String, GameStats>) {
    let mut stack = vec![TraversalStep::new(&tree)];
    while !stack.is_empty() {
        let step = stack.pop().unwrap();
        // println!("{:?}", step.tree.prefix().clone().as_bytes().to_vec());
        for child in step.tree.children() {
            let prefix = step.prefix_acc.clone().join(child.prefix());
            let prefix_vec = &prefix.as_bytes().to_vec()[step.offset..];
            // TODO: from the slice of the prefix vec, take up until the last SEPARATOR
            // The index of the last SEPARATOR is then the new offset
            // Everything before that should be parsed into moves
            // and then to `Chess` from the last `Chess` on the game stack,
            // then all pushed to the stack

            // prefix_vec.iter().position();
            stack.push(TraversalStep::build_step(
                child,
                step.game_stack.clone(),
                prefix,
                1, //FIXME with the calced offset of the last SEPARATOR
            ));
        }
    }
}
