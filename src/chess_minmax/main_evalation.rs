use chess::{BitBoard, Board, Color, Piece, ALL_FILES, ALL_RANKS, NUM_RANKS};

pub mod piece_square_tables;
use piece_square_tables::*;

pub fn evaluation_pieces_worth_plus(board: &Board) -> i16 {
    let white = board.color_combined(Color::White);
    let black = board.color_combined(Color::Black);

    let pawn = board.pieces(Piece::Pawn);
    let bishop = board.pieces(Piece::Bishop);
    let rook = board.pieces(Piece::Rook);
    let knight = board.pieces(Piece::Knight);
    let queen = board.pieces(Piece::Queen);
    let king = board.pieces(Piece::King);

    let delta_piece_table = |piece_bb: &BitBoard, w_table: &[i16; 64], b_table: &[i16; 64]| {
        weighted_sum(piece_bb & white, w_table) - weighted_sum(piece_bb & black, b_table)
    };

    let delta_pawn_p = delta_piece_table(pawn, &WHITE_PAWN, &BLACK_PAWN);
    let delta_rook_p = delta_piece_table(rook, &WHITE_ROOK, &BLACK_ROOK);
    let delta_bishop_p = delta_piece_table(bishop, &WHITE_BISHOP, &BLACK_BISHOP);
    let delta_knight_p = delta_piece_table(knight, &WHITE_KNIGHT, &BLACK_KNIGHT);

    // explicit calculation so we can use result to compute is_end_game
    let white_queen_p = weighted_sum(queen & white, &WHITE_QUEEN);
    let black_queen_p = weighted_sum(queen & black, &BLACK_QUEEN);
    let delta_queen_p = white_queen_p - black_queen_p;

    let delta_king_p = if white_queen_p == 0 && black_queen_p == 0 {
        delta_piece_table(king, &WHITE_KING_ENDGAME, &BLACK_KING_ENDGAME)
    } else {
        delta_piece_table(king, &WHITE_KING_MIDDLE, &BLACK_KING_MIDDLE)
    };

    delta_queen_p + delta_rook_p + delta_bishop_p + delta_knight_p + delta_pawn_p + delta_king_p
}
