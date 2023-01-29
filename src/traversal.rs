use crate::GameStats;
use radix_trie::{Trie, SubTrie, TrieCommon};

struct TraversalStep<'a> {
    tree: SubTrie<'a, String, GameStats>
}

pub fn extract_stats(tree: Trie<String, GameStats>) {
    let subtrie = tree.children().next().unwrap();
    let mut stack = vec![TraversalStep { tree: subtrie }];
    while !stack.is_empty() {
        let step = stack.pop().unwrap();
        for child in step.tree.children() {
            stack.push(TraversalStep { tree: child });
        }
    }
}
