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

pub fn extract_stats(tree: Trie<String, GameStats>) {
    let mut stack = vec![TraversalStep::new(&tree)];
    let mut pos_stats: HashMap<Chess, GameStats> = HashMap::new();
    while !stack.is_empty() {
        let mut step = stack.pop().unwrap();
        // println!("{:?}", step.tree.prefix().clone().as_bytes().to_vec());
        for child in step.tree.children() {
            let prefix = step.prefix_acc.clone().join(child.prefix());
            let prefix_vec = &prefix.as_bytes().to_vec()[step.offset..];
            let new_offset = prefix_vec.iter().rposition(|x| *x == SEPARATOR);

            // TODO: write with and_then!
            if let Some(end) = new_offset {
                if end > step.offset {
                    if let Ok(moves_string) = String::from_utf8(
                        prefix_vec[step.offset..end].to_vec()
                    ) {
                        for m_str in moves_string.split_whitespace() {
                            if let Some(new_pos) = step.game_stack.last() {
                                let san_may: Result<San, _> = m_str.parse();
                                if let Ok(san_move) = san_may {
                                    let move_may: Result<Move, _> =
                                        san_move.to_move(&new_pos.clone());
                                    if let Ok(m) = move_may {
                                        if new_pos.clone().play(&m).is_ok() {
                                            let new_pos = new_pos.clone();
                                            step.game_stack.push(new_pos);
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

            if let Some(game_stats) = step.tree.value() {
                for pos in &step.game_stack {
                    pos_stats
                        .entry(pos.clone())
                        .or_insert(GameStats::new())
                        .combine(&game_stats);
                }
            }

            stack.push(TraversalStep::build_step(
                child,
                step.game_stack.clone(), //figure out how to not clone?
                prefix,
                new_offset.unwrap_or(step.offset),
            ));
        }
    }
}
