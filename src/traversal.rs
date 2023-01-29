use crate::GameStats;
use nibble_vec::Nibblet;
use radix_trie::{Trie, SubTrie, TrieCommon};
use shakmaty::{Chess, Position, san::San, Move};
use std::collections::HashMap;

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
        let child = tree.children().next().unwrap();
        let prefix = child.prefix().clone();
        Self::build_step(
            child,
            vec![Chess::new()],
            prefix,
            0,
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

pub fn extract_stats(
    tree: Trie<String, GameStats>
) -> HashMap<Chess, GameStats> {
    let mut stack = vec![TraversalStep::new(&tree)];
    let mut pos_stats: HashMap<Chess, GameStats> = HashMap::new();
    while !stack.is_empty() {
        let step = stack.pop().unwrap();
        for child in step.tree.children() {
            let prefix = step.prefix_acc.clone().join(child.prefix());
            let prefix_vec = &prefix.as_bytes().to_vec()[step.offset..];
            let new_offset = prefix_vec.iter().rposition(|x| *x == SEPARATOR);
            let mut game_stack = step.game_stack.clone();

            // FIXME: only shows the first move for each branch...
            // TODO: write with and_then!
            if let Some(end) = new_offset {
                if end > step.offset {
                    if let Ok(moves_string) = String::from_utf8(
                        prefix_vec[step.offset..end].to_vec()
                    ) {
                        for m_str in moves_string.split_whitespace() {
                            if let Some(new_pos) = game_stack.last() {
                                let san_may: Result<San, _> = m_str.parse();
                                if let Ok(san_move) = san_may {
                                    let move_may: Result<Move, _> =
                                        san_move.to_move(&new_pos.clone());
                                    if let Ok(m) = move_may {
                                        if let Ok(new_pos) = new_pos.clone().play(&m) {
                                            game_stack.push(new_pos);
                                        } else {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(game_stats) = child.value() {
                for pos in &game_stack {
                    let base_stats = match pos_stats.get(pos) {
                        Some(stats) => *stats,
                        None => GameStats::new(),
                    };
                    pos_stats.insert(pos.clone(), base_stats.combine(game_stats));
                }
            }

            stack.push(TraversalStep::build_step(
                child,
                game_stack, //figure out how to not clone?
                prefix,
                new_offset.unwrap_or(step.offset),
            ));
        }
    }
    pos_stats
}
