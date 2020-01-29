use chess::{
    get_file, Board, BoardStatus, ChessMove, Color, MoveGen, Piece, ALL_FILES, ALL_RANKS, NUM_FILES,
};
use rand::Rng;
use std::f64::{INFINITY, NEG_INFINITY};

// TODO: add transposition table
// https://en.wikipedia.org/wiki/Negamax#Negamax_with_alpha_beta_pruning_and_transposition_tables
// TODO: use fast hash for transposition table hashing

fn negamax(board: &Board, depth: u32, a: f64, b: f64, rng: &mut impl Rng) -> f64 {
    let mut a = a;

    let color_index = match board.side_to_move() {
        Color::White => 1,
        Color::Black => -1,
    };

    if depth == 0 {
        return f64::from(color_index) * evaluation_fn(board, rng);
    }

    if board.status() != BoardStatus::Ongoing {
        return f64::from(color_index) * (stats_eval_fn(board.status(), color_index, depth));
    }

    let child_nodes = MoveGen::new_legal(&board).map(|mov| board.make_move_new(mov));

    let mut value = NEG_INFINITY;
    for child in child_nodes {
        let node_eval = -negamax(&child, depth - 1, -b, -a, rng);

        value = f64::max(value, node_eval);
        a = f64::max(a, value);
        if a >= b {
            break;
        }
    }
    value
}

pub fn negamax_prelude(board: &Board, depth: u32, rng: &mut impl Rng) -> Option<(ChessMove, f64)> {
    let mut a = NEG_INFINITY;
    let b = INFINITY;

    if board.status() != BoardStatus::Ongoing {
        return None;
    }

    let child_nodes = MoveGen::new_legal(&board).map(|mov| (mov, board.make_move_new(mov)));

    let mut value = NEG_INFINITY;
    let mut best_mov = None;
    for (mov, child) in child_nodes {
        let node_eval = -negamax(&child, depth - 1, -b, -a, rng);

        if node_eval >= value {
            value = node_eval;
            best_mov = Some(mov);
        }
        a = f64::max(a, value);

        if a >= b {
            break;
        }
    }

    if best_mov.is_none() {
        println!("\nNone End: {}", board);
    }

    best_mov.map(|mov| (mov, value))
}

fn stats_eval_fn(stats: BoardStatus, color_index: i8, depth: u32) -> f64 {
    const CHECKMATE_SCORE: f64 = 2e5; // base score when checkmated
    // additional score for each depth when checkmated to encourage faster checkmate
    //
    // This should be large enough to compensate pieces value
    // else AI won't do sacrifice for checkmate and AI will prefer eating all the enemy pieces than fast win
    // which presumably we don't want
    const CHECKMATE_DEPTH_SCORE: f64 = 1e3;

    match stats {
        BoardStatus::Ongoing => {
            unreachable!("Ongoing game shouldn't be able to call this function")
        }
        BoardStatus::Stalemate => 0.0,
        BoardStatus::Checkmate => {
            f64::from(color_index) * -(CHECKMATE_SCORE + CHECKMATE_DEPTH_SCORE * f64::from(depth))
        }
    }
}

fn evaluation_fn(board: &Board, rng: &mut impl Rng) -> f64 {
    // this function is call after move simulation so board.side_to_move() == enemy side
    // higher = better for white

    const NOISE_FACTOR: f64 = 1e-4;
    let tiny_noise = rng.gen_range(-NOISE_FACTOR, NOISE_FACTOR);
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

fn evaluation_pieces_worth_plus(board: &Board) -> f64 {
    let white = board.color_combined(Color::White);
    let black = board.color_combined(Color::Black);

    let pawn = board.pieces(Piece::Pawn);
    let bishop = board.pieces(Piece::Bishop);
    let rook = board.pieces(Piece::Rook);
    let knight = board.pieces(Piece::Knight);
    let queen = board.pieces(Piece::Queen);

    let white_pawn = pawn & white;
    let black_pawn = pawn & black;

    let delta_pawn = f64::from((white_pawn).popcnt()) - f64::from((black_pawn).popcnt());
    let delta_rook = f64::from((rook & white).popcnt()) - f64::from((rook & black).popcnt());
    let delta_queen = f64::from((queen & white).popcnt()) - f64::from((queen & black).popcnt());
    let delta_bishop = f64::from((bishop & white).popcnt()) - f64::from((bishop & black).popcnt());
    let delta_knight = f64::from((knight & white).popcnt()) - f64::from((knight & black).popcnt());

    let mut white_stack_pawn = 0;
    let mut black_stack_pawn = 0;
    for file in ALL_FILES.iter() {
        let file_bit_board = get_file(*file);
        let white_count = (file_bit_board & white_pawn).popcnt();
        let black_count = (file_bit_board & black_pawn).popcnt();

        // if double pawn
        if white_count > 1 {
            white_stack_pawn += white_count - 1
        }
        if black_count > 1 {
            black_stack_pawn += black_count - 1
        }
    }
    let delta_stack_pawn = f64::from(white_stack_pawn) - f64::from(black_stack_pawn);

    9.0 * delta_queen + 5.0 * delta_rook + 3.0 * (delta_bishop + delta_knight) + 1.0 * delta_pawn
        - 0.5 * delta_stack_pawn
}

#[cfg(test)]
mod tests {

    use super::negamax_prelude;
    use chess::{Board, ChessMove, Color, File, Rank, Square};
    use rand::thread_rng;
    use std::str::FromStr;

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
            let (_, score) = negamax_prelude(&board, 5, rng).unwrap();

            let guess = if score > 0.0 { player } else { !player };

            assert_eq!(guess, *answer);
        }
    }
}
