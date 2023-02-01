use crate::chess_db;
use crate::game_stats::GameWins;
use nibble_vec::Nibblet;
use radix_trie::{SubTrie, Trie, TrieCommon};
use rocksdb::{WriteBatch, DB};
use shakmaty::{san::San, Chess, Move, Position};

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
        let child = tree.children().next()?;
        let prefix = child.prefix().clone();
        Some(Self::build_step(
            child,
            vec![Game {
                position: Chess::new(),
                game_move: None,
            }],
            prefix,
            0,
        ))
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

fn is_true_leaf(tree: &StatsST) -> bool {
    let mut ret = true;
    for _ in tree.children() {
        ret = false;
    }
    ret
}

//FIXME -- still lost on empty moves_string in the leaf case...
pub fn extract_stats(db: &DB, tree: &mut Trie<String, GameWins>) {
    let tree = std::mem::take(tree); // Clearing out the tree for later use
    let mut batch = WriteBatch::default();
    let mut stack = match TraversalStep::new(&tree) {
        Some(step) => vec![step],
        None => vec![],
    };
    while !stack.is_empty() {
        let step = stack
            .pop()
            .expect("should only loop if the stack has items");
        if is_true_leaf(&step.tree) {
            // println!("true leaf!!");
            let prefix = step.prefix_acc.clone();
            let mut game_stack = step.game_stack.clone();

            let moves_string = step
                .tree
                .key()
                .expect("invalid key for tree?")
                .chars()
                .into_iter()
                .skip(
                    String::from_utf8(prefix.clone().into_bytes())
                        .expect("Invalid utf8 for prefix")
                        .chars()
                        .count(),
                )
                .collect::<String>();

            // println!("Prefix: {:?}",
            //         String::from_utf8(prefix.clone().into_bytes())
            //             .expect("Invalid utf8 for prefix"));
            // println!("Key: {:?}", step.tree.key());
            // println!("Moves string: {}", moves_string);
            // println!("Games Stack: {:?}", game_stack);
            for m_str in moves_string.split_whitespace() {
                if let Some(Game {
                    position: old_pos,
                    game_move: _,
                }) = game_stack.pop()
                {
                    let san_may: Result<San, _> = m_str.parse();
                    if let Ok(san_move) = san_may {
                        let move_may: Result<Move, _> =
                            san_move.to_move(&old_pos.clone());
                        if let Ok(m) = move_may {
                            if let Ok(new_pos) = old_pos.clone().play(&m) {
                                game_stack.push(Game {
                                    position: old_pos,
                                    game_move: Some(m),
                                });
                                game_stack.push(Game {
                                    position: new_pos,
                                    game_move: None,
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

            if let Some(game_stats) = step.tree.value() {
                for game in game_stack.iter().rev() {
                    chess_db::update_pos_wins(
                        db,
                        &mut batch,
                        game.position.clone(),
                        *game_stats,
                    );
                    if let Some(m) = game.game_move.clone() {
                        chess_db::update_pos_move_wins(
                            db,
                            &mut batch,
                            game.position.clone(),
                            m,
                            *game_stats,
                        )
                    }
                }
                let old_batch = std::mem::take(&mut batch);
                db.write(old_batch).expect("Batch couldn't write to DB");
            }
        } else {
            // iterate through children in trie
            for child in step.tree.children() {
                // build up the new moves
                let prefix = step.prefix_acc.clone().join(child.prefix());
                let prefix_vec = &prefix.as_bytes().to_vec()[step.offset..];
                let new_offset =
                    prefix_vec.iter().rposition(|x| *x == SEPARATOR);
                let mut game_stack = step.game_stack.clone();

                // TODO: write with and_then!
                if let Some(end) = new_offset {
                    if end > step.offset {
                        if let Ok(moves_string) = String::from_utf8(
                            // extract the new moves
                            prefix_vec[..end].to_vec(),
                        ) {
                            // println!("Start index: {}", step.offset);
                            // println!("End index: {}", end);
                            // println!(
                            //     "Last position: {}",
                            //     chess_db::pos_to_fen(game_stack.last().unwrap().position.clone())
                            // );
                            // println!("Moves string: {}", moves_string);
                            // println!(
                            //     "Moves string unindexed: {}",
                            //     String::from_utf8(prefix_vec.to_vec()).unwrap(),
                            // );
                            for m_str in moves_string.split_whitespace() {
                                // pop the old game
                                if let Some(Game {
                                    position: old_pos,
                                    game_move: _,
                                }) = game_stack.pop()
                                {
                                    // parse the move
                                    let san_may: Result<San, _> = m_str.parse();
                                    if let Ok(san_move) = san_may {
                                        // println!(
                                        //     "Old pos: {}",
                                        //     chess_db::pos_to_fen(old_pos.clone()),
                                        // );
                                        // println!("SAN move: {:?}", san_move);
                                        if san_move.to_string() == "b6" {
                                            println!(
                                                "Game stack: {:?}",
                                                game_stack,
                                            );
                                            println!(
                                                "Old pos: {}",
                                                chess_db::pos_to_fen(old_pos.clone()),
                                            );
                                        }
                                        let move_may: Result<Move, _> =
                                            san_move.to_move(&old_pos.clone());
                                        if let Ok(m) = move_may {
                                            // FIXME - doesn't match pos
                                            // check that the move actually works
                                            // println!("Accepted move: {:?}", m);
                                            if let Ok(new_pos) =
                                                old_pos.clone().play(&m)
                                            {
                                                game_stack.push(Game {
                                                    position: old_pos,
                                                    game_move: Some(m),
                                                });
                                                game_stack.push(Game {
                                                    position: new_pos,
                                                    game_move: None,
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
                    for game in game_stack.iter().rev() {
                        chess_db::update_pos_wins(
                            db,
                            &mut batch,
                            game.position.clone(),
                            *game_stats,
                        );
                        if let Some(m) = game.game_move.clone() {
                            chess_db::update_pos_move_wins(
                                db,
                                &mut batch,
                                game.position.clone(),
                                m,
                                *game_stats,
                            )
                        }
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
    }
    db.write(batch).expect("Batch couldn't write to DB");
}
