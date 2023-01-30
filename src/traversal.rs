use game_stats::GameStats;
use nibble_vec::Nibblet;
use radix_trie::{Trie, SubTrie, TrieCommon};
use shakmaty::{Chess, Position, san::San, Move};
use std::collections::HashMap;

const SEPARATOR: u8 = 32;

type StatsTree = Trie<String, GameStats>;
type StatsST<'a> = SubTrie<'a, String, GameStats>;

#[derive(Debug, Clone)]
struct Game {
    position: Chess,
    game_move: Option<Move>,
}

#[derive(Debug)]
struct TraversalStep<'a> {
    tree: StatsST<'a>,
    game_stack: Vec<Game>,
    prefix_acc: Nibblet,
    offset: usize,
}

impl TraversalStep<'_> {
    fn new(tree: &StatsTree) -> TraversalStep {
        let child = tree.children().next().unwrap();
        let prefix = child.prefix().clone();
        Self::build_step(
            child,
            vec![Game{ position: Chess::new(), game_move: None}],
            prefix,
            0,
        )
    }

    fn build_step(
        tree: StatsST,
        game_stack: Vec<Game>,
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
    let mut pos_moves: HashMap<Chess, HashMap<Move, u32>> = HashMap::new();
    while !stack.is_empty() {
        let step = stack.pop().unwrap();
        for child in step.tree.children() {
            let prefix = step.prefix_acc.clone().join(child.prefix());
            let prefix_vec = &prefix.as_bytes().to_vec()[step.offset..];
            let new_offset = prefix_vec.iter().rposition(|x| *x == SEPARATOR);
            let mut game_stack = step.game_stack.clone();

            // TODO: write with and_then!
            if let Some(end) = new_offset {
                if end > step.offset {
                    if let Ok(moves_string) = String::from_utf8(
                        prefix_vec[step.offset..end].to_vec()
                    ) {
                        for m_str in moves_string.split_whitespace() {
                            if let Some(Game {
                                position: new_pos,
                                game_move: _,
                            }) = game_stack.last() {
                                let san_may: Result<San, _> = m_str.parse();
                                if let Ok(san_move) = san_may {
                                    let move_may: Result<Move, _> =
                                        san_move.to_move(&new_pos.clone());
                                    if let Ok(m) = move_may {
                                        if let Ok(new_pos) = new_pos.clone().play(&m) {
                                            game_stack.push(Game {
                                                position: new_pos,
                                                game_move: Some(m),
                                            });
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
                let mut move_may: Option<Move> = None;
                for game in game_stack.iter().rev() {
                    let base_stats = match pos_stats.get(&game.position) {
                        Some(stats) => *stats,
                        None => GameStats::new(),
                    };
                    // insert wins by color
                    pos_stats.insert(game.position.clone(), base_stats.combine(game_stats));

                    // insert moves by position
                    if let Some(m) = move_may.clone() {
                        match pos_moves.get_mut(&game.position) {
                            Some(move_counts) => match move_counts.get_mut(&m) {
                                Some(count) => {
                                    *count += game_stats.total();
                                }
                                None => {
                                    let m_clone = m.clone();
                                    move_counts.insert(m_clone, game_stats.total());
                                }
                            }
                            None => {
                                let pos_clone = game.position.clone();
                                pos_moves.insert(pos_clone, HashMap::from([(m, game_stats.total())]));
                            }
                        }
                    }
                    move_may = game.game_move.clone();
                }
            }

            stack.push(TraversalStep::build_step(
                child,
                game_stack,
                prefix,
                new_offset.unwrap_or(step.offset),
            ));
        }
    }
    pos_stats
}
