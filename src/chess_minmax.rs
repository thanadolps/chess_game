use chess::{get_file, Board, BoardStatus, ChessMove, Color, MoveGen, Piece, ALL_FILES, ALL_RANKS, NUM_RANKS, get_rank, EMPTY};
use rand::Rng;
use std::f64::{INFINITY, NEG_INFINITY};
use lru::LruCache;
use std::hash::{Hash, Hasher, BuildHasher};
use crate::chess_minmax::main_evalation::evaluation_pieces_worth_plus;
use itertools::Itertools;

pub mod main_evalation;

// TODO: add transposition table
// https://en.wikipedia.org/wiki/Negamax#Negamax_with_alpha_beta_pruning_and_transposition_tables
// TODO: use fast hash for transposition table hashing

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct BoardHash(u64);
impl BoardHash {
    pub fn new(board: &Board) -> BoardHash {
        BoardHash(board.get_hash())
    }
}


pub enum BoundedScore {
    LowerBound(i16),
    UpperBound(i16),
    Exact(i16)
}

pub struct TranspositionItem {
    score: BoundedScore,
    depth: u8
}

fn negamax<K: BuildHasher>(board: &Board, depth: u8, mut a: i16, mut b: i16, rng: &mut impl Rng, cache: &mut LruCache<BoardHash, TranspositionItem, K>) -> i16 {
    // var setup
    let a_orig = a;

    let color_index = match board.side_to_move() {
        Color::White => 1,
        Color::Black => -1,
    };

    // terminating condition
    if depth == 0 {
        return color_index as i16 * evaluation_fn(board, rng);
    }
/*
    // Cache checking
    let board_hash = BoardHash::new(board);
    if let Some(tt_entry) =
        cache.get(&board_hash).filter(|tte| tte.depth >= depth) {

        let entry_val = match tt_entry.score {
            BoundedScore::Exact(ex) => { return ex},
            BoundedScore::LowerBound(lb) => { a = i16::max(a, lb); lb },
            BoundedScore::UpperBound(ub) => { b = i16::min(b, ub); ub },
        };

        if a >= b {
            return entry_val
        }
    }*/

    // negamax core
    let child_nodes = MoveGen::new_legal(&board).map(|mov| board.make_move_new(mov));

    let mut value = -i16::MAX;
    for child in child_nodes {
        let node_eval = -negamax(&child, depth - 1, -b, -a, rng, cache);
        debug_assert!(node_eval > -i16::MAX);
        value = i16::max(value, node_eval);

        a = i16::max(a, value);
        if a >= b {
            break;
        }
    }

    debug_assert_eq!(value == -i16::MAX, board.status() != BoardStatus::Ongoing);
    // terminating condition 2 (no move)
    if value == -i16::MAX {
        let status = if *board.checkers() == EMPTY {
            BoardStatus::Stalemate
        } else {
            BoardStatus::Checkmate
        };
        return color_index as i16 * stats_eval_fn(status, color_index, depth)
    }

/*
    // Cache store
    let new_entry_score =
    if value <= a_orig {
        BoundedScore::UpperBound(value)
    }
    else if value >= b {
        BoundedScore::LowerBound(value)
    }
    else {
        BoundedScore::Exact(value)
    };
    let new_entry = TranspositionItem {
        score: new_entry_score,
        depth
    };
    cache.put(board_hash, new_entry);*/

    // Returning
    value
}

pub fn negamax_prelude<K: BuildHasher>(board: &Board, depth: u8, rng: &mut impl Rng, cache: &mut LruCache<BoardHash, TranspositionItem, K>) -> Option<(ChessMove, i16)> {
    // var initialization
    let mut a = -i16::MAX;  // don't use i16::MIN! it will overflow on negation
    let b = i16::MAX;
    dbg!(depth);
    // cache check doesn't provide move so it's unusable here

    // negamax
    let child_nodes = MoveGen::new_legal(&board).map(|mov| (mov, board.make_move_new(mov)));

    let mut value = -i16::MAX;
    let mut best_mov = None;

    for (mov, child) in child_nodes {
        let node_eval = -negamax(&child, depth - 1, -b, -a, rng, cache);

        if node_eval > value {
            value = node_eval;
            best_mov = Some(mov);
        }
        a = i16::max(a, value);

        if a >= b {
            break;
        }
    }
    /*
    // Cache store
    let new_entry_score =
        // alpha case optimized out cause a_orig is -inf
        if value <= -i16::MAX {
            BoundedScore::UpperBound(value)
        }
        else if value >= b {
            BoundedScore::LowerBound(value)
        }
        else {
            BoundedScore::Exact(value)
        };
    let new_entry = TranspositionItem {
        score: new_entry_score,
        depth
    };
    cache.put(BoardHash::new(board), new_entry);*/

    // Returning
    if best_mov.is_none() {
        println!("\nNone End: {}", board);
    }

    best_mov.map(|mov| (mov, value))
}

fn stats_eval_fn(stats: BoardStatus, color_index: i8, depth: u8) -> i16 {
    const CHECKMATE_SCORE: i16 = 20000; // base score when checkmated
                                      // additional score for each depth when checkmated to encourage faster checkmate
                                      //
                                      // This should be large enough to compensate pieces value
                                      // else AI won't do sacrifice for checkmate and AI will prefer eating all the enemy pieces than fast win
                                      // which presumably we don't want
                                      // beware of limit of i16 (max 32767)
    // DEPTH * CHECKMATE_DEPTH_SCORE shall never exceed 8000
    // else overflow will happen
    const CHECKMATE_DEPTH_SCORE: i16 = 500; // approximately rook

    match stats {
        BoardStatus::Ongoing => {
            unreachable!("Ongoing game shouldn't be able to call this function")
        }
        BoardStatus::Stalemate => 0,
        BoardStatus::Checkmate => {
            color_index as i16 * -(CHECKMATE_SCORE + CHECKMATE_DEPTH_SCORE * depth as i16)
        }
    }
}

fn evaluation_fn(board: &Board, rng: &mut impl Rng) -> i16 {
    // this function is call after move simulation so board.side_to_move() == enemy side
    // higher = better for white

    let tiny_noise = rng.gen_range(-1, 2);
    evaluation_pieces_worth_plus(board) + tiny_noise
}

fn evaluation_count_pieces(board: &Board) -> f64 {
    let white_pieces_count = f64::from(board.color_combined(Color::White).popcnt());
    let black_pieces_count = f64::from(board.color_combined(Color::Black).popcnt());
    white_pieces_count - black_pieces_count
}

fn evaluation_reverse_count_pieces(board: &Board) -> f64 {
    let white_pieces_count = f64::from(board.color_combined(Color::White).popcnt());
    let black_pieces_count = f64::from(board.color_combined(Color::Black).popcnt());
    black_pieces_count - white_pieces_count
}

fn evaluation_pawn_motion(board: &Board) -> f64 {
    let white = board.color_combined(Color::White);
    let black = board.color_combined(Color::Black);
    let pawn = board.pieces(Piece::Pawn);

    let white_pwn = (pawn & white)
        .map(|sq| sq.get_rank().to_index())
        .sum::<usize>() as f64;
    let black_pwn = (pawn & black)
        .map(|sq| sq.get_rank().to_index())
        .sum::<usize>() as f64;

    white_pwn - black_pwn
}

fn evaluation_freedom(board: &Board) -> f64 {
    MoveGen::new_legal(&board)
        .map(|mov| {
            let moving_color = board.color_on(mov.get_source()).unwrap();
            match moving_color {
                Color::White => 1.0,
                Color::Black => -1.0,
            }
        })
        .sum()
}

fn evaluation_pieces_worth(board: &Board) -> f64 {
    let white = board.color_combined(Color::White);
    let black = board.color_combined(Color::Black);

    let pawn = board.pieces(Piece::Pawn);
    let bishop = board.pieces(Piece::Bishop);
    let rook = board.pieces(Piece::Rook);
    let knight = board.pieces(Piece::Knight);
    let queen = board.pieces(Piece::Queen);

    let delta_pawn = f64::from((pawn & white).popcnt()) - f64::from((pawn & black).popcnt());
    let delta_rook = f64::from((rook & white).popcnt()) - f64::from((rook & black).popcnt());
    let delta_queen = f64::from((queen & white).popcnt()) - f64::from((queen & black).popcnt());
    let delta_bishop = f64::from((bishop & white).popcnt()) - f64::from((bishop & black).popcnt());
    let delta_knight = f64::from((knight & white).popcnt()) - f64::from((knight & black).popcnt());

    9.0 * delta_queen + 5.0 * delta_rook + 3.0 * (delta_bishop + delta_knight) + 1.0 * delta_pawn
}

#[cfg(test)]
mod tests {

    use super::negamax_prelude;
    use chess::{Board, ChessMove, Color, File, Rank, Square};
    use rand::thread_rng;
    use std::str::FromStr;
    use lru::LruCache;

    fn build_move(file1: File, rank1: Rank, file2: File, rank2: Rank) -> ChessMove {
        ChessMove::new(
            Square::make_square(rank1, file1),
            Square::make_square(rank2, file2),
            None,
        )
    }

    #[test]
    fn test_who_good() {
        let question = [
            (
                "5r1k/7p/q1p3p1/2bp3n/8/P1N5/BPP2PPP/R4QK1 b - - 0 1",
                Color::Black,
            ),
            (
                "r2b1rk1/2pq2p1/1p4P1/1Pnnpp2/p1P5/P2PPP2/1B3P2/2KQ2RR w - - 0 1",
                Color::Black,
            ),
        ];

        let rng = &mut thread_rng();

        for (fen, answer) in question.iter() {
            let board = Board::from_str(fen).unwrap();
            let player = board.side_to_move();
            let mut cache = LruCache::new(64);
            let (_, score) = negamax_prelude(&board, 5, rng, &mut cache).unwrap();

            let guess = if score > 0 { player } else { !player };

            assert_eq!(guess, *answer);
        }
    }
}
