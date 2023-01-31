use crate::game_stats::GameWins;
use nibble_vec::Nibblet;
use radix_trie::{Trie, SubTrie, TrieCommon};
use rocksdb::{DB, WriteBatch};
use shakmaty::{Chess, Position, san::San, Move};
use crate::chess_db;

const SEPARATOR: u8 = 32;

type StatsST<'a> = SubTrie<'a, String, GameWins>;

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
    fn new(tree: &Trie<String, GameWins>) -> Option<TraversalStep> {
        match tree.children().next() {
            Some(child) => {
                let prefix = child.prefix().clone();
                Some(Self::build_step(
                    child,
                    vec![Game{ position: Chess::new(), game_move: None}],
                    prefix,
                    0,
                ))
            }
            None => {
                println!("Attempted to start a traversal without any children!");
                dbg!({}, tree);
                None
            }
        }
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
    db: &DB,
    tree: &mut Trie<String, GameWins>
) {
    let tree = std::mem::take(tree); // Clearing out the tree for later use
    let mut batch = WriteBatch::default();
    let mut stack = match TraversalStep::new(&tree) {
        Some(step) => vec![step],
        None       => vec![],
    };
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
                    chess_db::update_pos_wins(
                        db,
                        &mut batch,
                        game.position.clone(),
                        *game_stats,
                    );
                    if let Some(m) = move_may.clone() {
                        chess_db::update_pos_move_wins(
                            db,
                            &mut batch,
                            game.position.clone(),
                            m,
                            *game_stats,
                        )
                    }
                    move_may = game.game_move.clone();
                }
                let old_batch = std::mem::take(&mut batch);
                db.write(old_batch).expect("Batch couldn't write to DB");
            }

            stack.push(TraversalStep::build_step(
                child,
                game_stack,
                prefix,
                new_offset.unwrap_or(step.offset),
            ));
        }
    }
    db.write(batch).expect("Batch couldn't write to DB");
}
