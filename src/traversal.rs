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

fn extract_san_strs(step: &TraversalStep, node: &StatsST) -> (Nibblet, usize, Vec<String>) {
    let prefix = step.prefix_acc.clone().join(node.prefix());
    let pre_offset_vec = &prefix.as_bytes().to_vec();
    let prefix_vec = &prefix.as_bytes().to_vec();
    let new_offset = if !step.tree.is_leaf() && prefix_vec.last() == Some(&SEPARATOR) {
        prefix_vec.iter().rev().skip(1).rev().rposition(|x| *x == SEPARATOR)
    } else {
        prefix_vec.iter().rposition(|x| *x == SEPARATOR)
    };


    // println!("\n\nEXTRACT MOVES\n");
    // println!("Prefix: {prefix:?}");
    // println!("Pre-offset Vec: {pre_offset_vec:?}");
    // println!("Prefix Vec: {prefix_vec:?}");
    // println!("Offset: {:?}", step.offset);
    // println!("New Offset: {new_offset:?}");
    if let Some(end) = new_offset {
        if end > step.offset {
            if let Ok(moves_string) = String::from_utf8(prefix_vec[step.offset..end].to_vec()) {
                let full_moves = String::from_utf8(prefix_vec.to_vec()).unwrap();
                let san_strs: Vec<String> = moves_string.split_whitespace().map(|s| s.to_string()).collect();
                // if full_moves.starts_with(" e4 c6 d4 d5 Nc3 dxe4 Nxe4 Nf6 Nxf6 exf6 c3 Be6 Bd3 Bd6 Qc2 h6 Ne2 Qc7 g3 Nd7 Bf4 O-O") {
                //     println!("Prefix acc: {:?}", step.prefix_acc);
                //     println!("Prefix: {:?}", prefix);
                //     println!("Prefix vec: {:?}", prefix_vec.to_vec());
                //     println!("New offset: {}", end);
                //     println!("Full Moves String: {}", full_moves);
                //     println!("Truncated Moves String: {}", moves_string);
                // }
                (prefix, end, san_strs)
            } else {
                (prefix, end, Vec::new())
            }
        } else {
            (prefix, step.offset, Vec::new())
        }
    } else {
        (prefix, step.offset, Vec::new())
    }
}

pub fn extract_stats(db: &DB, tree: &mut Trie<String, GameWins>) {
    let tree = std::mem::take(tree); // Clearing out the tree for later use
    let mut batch = WriteBatch::default();
    let mut stack = match TraversalStep::new(&tree) {
        Some(step) => vec![step],
        None => vec![],
    };

    while !stack.is_empty() {
        //extract current step
        let step = stack
            .pop()
            .expect("should only loop if the stack has items");
        for child in step.tree.children() {
            let mut game_stack = step.game_stack.clone();
            // println!("Child in loop: {:?}", child);
            let (prefix, offset, san_strs) = extract_san_strs(&step, &child);
            // partial castle showing up here "-O"
            for san_str in san_strs.clone() {
                let Game {position: old_pos, game_move: _} =
                    game_stack.pop().expect("No step on top of game stack!!");
                // println!("Pos: {:?}", chess_db::pos_to_fen(old_pos.clone()));
                // println!("Moves: {:?}", san_strs);
                // if san_str == "-O" {
                //     println!("prefix: {:?}", prefix.clone());
                //     println!("offset: {}", offset.clone());
                //     println!("Key: {:?}", child.key());
                //     println!("Moves: {:?}", san_strs);
                //     println!("Move: {}", san_str);
                // }
                let san_move: San = san_str.parse().expect("Invalid SAN.");


                // FIXME: this block shows the issue in the tree!!
                // if chess_db::pos_to_fen(old_pos.clone()) == "rnbqkbnr/pp1p1ppp/8/2p1p3/3PP1P1/8/PPP2P1P/RNBQKBNR b KQkq -" && san_str == "c5" {
                //     println!("Key: {:?}", child.key());
                //     println!("Pos: {:?}", chess_db::pos_to_fen(old_pos.clone()));
                //     println!("Move: {}", san_str);
                // }

                // println!("\n\nMAKE MOVE\n");
                // println!("old prefix: {:?}", step.prefix_acc);
                // println!("old offset: {:?}", step.offset);
                // println!("new prefix: {:?}", prefix);
                // println!("new offset: {:?}", offset);
                // println!("Moves: {:?}", san_strs);
                // println!("Pos: {:?}", chess_db::pos_to_fen(old_pos.clone()));
                // println!("Move: {}", san_str);
                // println!("Next move: {}", String::from_utf8(vec![78, 102, 54]).unwrap());
                let m = san_move.to_move(&old_pos).expect("Invalid position/move combo");
                let new_pos = old_pos.clone().play(&m).expect("Invalid move for old pos!");
                game_stack.push(Game {
                    position: old_pos.clone(),
                    game_move: Some(m),
                });
                game_stack.push(Game {
                    position: new_pos.clone(),
                    game_move: None,
                });
                // if chess_db::pos_to_fen(new_pos.clone()) == "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq -" {
                    // println!("old prefix: {:?}", step.prefix_acc);
                    // println!("old offset: {:?}", step.offset);
                    // println!("new prefix: {:?}", prefix);
                    // println!("new offset: {:?}", offset);
                    // println!("Moves: {:?}", san_strs);
                    // println!("Move: {}", san_str);
                // }
            }
            if let Some(game_stats) = child.value() {
                for game in game_stack.iter() {
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
                game_stack.clone(),
                prefix,
                offset,
            ));
        }
    }
    db.write(batch).expect("Batch couldn't write to DB");
}
