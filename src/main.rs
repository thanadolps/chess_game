use chess::{BitBoard, Board, BoardStatus, ChessMove, Color, Game, MoveGen, Piece, Square};
use rand::{thread_rng, Rng};
use std::cmp::Ordering::Equal;
use std::f64::{INFINITY, NEG_INFINITY};
use std::time::{Duration, Instant};

use std::io::stdin;
use std::str::FromStr;

mod chess_minmax;
use chess_minmax::negamax_prelude;

mod chess_graphic;
use chess_graphic::ChessGraphic;

fn main() {
    graphic();
}

fn graphic() {
    use piston_window::*;

    let mut window: PistonWindow = WindowSettings::new("Chess?", (640, 480))
        .exit_on_esc(true)
        .build()
        .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    let mut game = ChessGraphic::new(&mut window.create_texture_context());
    window.set_max_fps(10);

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g, _| {
            game.draw(c, g);
        });

        if let Some(button) = e.press_args() {
            game.button_input(&button);
        }

        if let Some(mouse_pos) = e.mouse_cursor_args() {
            game.on_mouse_position(mouse_pos);
        }

        if let Some(resize_args) = e.resize_args() {
            game.on_resize(resize_args);
        }
    }
}

fn io() {
    loop {
        let mut input_str: String = String::new();

        stdin().read_line(&mut input_str).unwrap();

        let board = Board::from_str(&input_str).expect("Invalid FEN");
        let rng = &mut thread_rng();
        const DEPTH: u32 = 5;

        let result = negamax_prelude(&board, DEPTH, rng).unwrap();

        println!("{} {}", result.0, result.1);
    }
}

fn batch_generator() {
    // sicilian defense
    let mut game = Game::new();
    game.make_move(ChessMove::new(Square::E2, Square::E4, None));
    game.make_move(ChessMove::new(Square::C7, Square::C5, None));
    print!("1. e2e4 c7c5 {{Sicilian Defense}} ");
    game.make_move(ChessMove::new(Square::G1, Square::F3, None));
    game.make_move(ChessMove::new(Square::D7, Square::D6, None));
    print!("2. g1f3 d7d6 {{Sicilian Continue}} ");
    // fen
    // let mut game = Game::from_str("4k3/8/1p6/8/8/8/5QQQ/4K3 w - - 0 1").unwrap();

    const START_NUM: u32 = 4;
    const MAX_LENGTH: u32 = 150;

    const SCORE_DISPLAY_THRESHOLD: f64 = 1e5;

    const WHITE_DEPTH: u32 = 4;
    const BLACK_DEPTH: u32 = 4;

    let rng = &mut thread_rng();

    let start_time = Instant::now();

    for i in START_NUM..MAX_LENGTH + START_NUM {
        print!("{}. ", i);
        if let Some((mov, score)) = negamax_prelude(&game.current_position(), WHITE_DEPTH, rng) {
            game.make_move(mov);

            print!("{}{}", mov.get_source(), mov.get_dest());
            print!(
                "{} ",
                mov.get_promotion()
                    .map_or(String::new(), |x| x.to_string(Color::White))
            );

            if score < SCORE_DISPLAY_THRESHOLD || score > -SCORE_DISPLAY_THRESHOLD {
                print!("{{{:.2}}} ", score);
            } else if score > 0.0 {
                print!("{{inf}} ")
            } else {
                print!("{{-inf}}")
            }
        } else {
            break;
        }

        if let Some((mov, score)) = negamax_prelude(&game.current_position(), BLACK_DEPTH, rng) {
            game.make_move(mov);

            print!("{}{}", mov.get_source(), mov.get_dest());
            print!(
                "{} ",
                mov.get_promotion()
                    .map_or(String::new(), |x| x.to_string(Color::White))
            );

            if score < SCORE_DISPLAY_THRESHOLD || score > -SCORE_DISPLAY_THRESHOLD {
                print!("{{{:.2}}} ", score);
            } else if score > 0.0 {
                print!("{{inf}} ")
            } else {
                print!("{{-inf}}")
            }
        } else {
            break;
        }
    }

    let end_time = Instant::now();

    println!();
    println!("time used: {} ms", (end_time - start_time).as_millis());
}
