use chess::{Board, ChessMove, Color, Game, MoveGen, Piece, Square};
use rand::thread_rng;

use std::time::Instant;

use std::io::stdin;
use std::str::FromStr;

mod chess_minmax;
use chess_minmax::negamax_prelude;

mod chess_graphic;
use chess_graphic::ChessGraphic;
use lru::LruCache;
use std::fs::OpenOptions;

pub const CACHE_SIZE: usize = 4096;

fn main() {
    graphic();
    // batch_generator();
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
/*
fn io() {
    loop {
        let mut input_str: String = String::new();

        stdin().read_line(&mut input_str).unwrap();

        let board = Board::from_str(&input_str).expect("Invalid FEN");
        let rng = &mut thread_rng();
        let mut cache = LruCache::new(CACHE_SIZE);

        const DEPTH: u8 = 5;

        let result = negamax_prelude(&board, DEPTH, rng, &mut cache).unwrap();

        println!("{} {}", result.0, result.1);
    }
}

fn batch_generator() {
    use std::io::Write;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(concat!(env!("CARGO_MANIFEST_DIR"), "/chess.txt"))
        .unwrap();

    // sicilian defense
    let mut game = Game::new();
    game.make_move(ChessMove::new(Square::E2, Square::E4, None));
    game.make_move(ChessMove::new(Square::C7, Square::C5, None));
    write!(file, "e2e4 c7c5 {{Sicilian Defense}} ").unwrap();
    game.make_move(ChessMove::new(Square::G1, Square::F3, None));
    game.make_move(ChessMove::new(Square::D7, Square::D6, None));
    write!(file, "g1f3 d7d6 {{Sicilian Continue}} ").unwrap();
    // fen
    // let mut game = Game::from_str("4k3/8/1p6/8/8/8/5QQQ/4K3 w - - 0 1").unwrap();

    const START_NUM: u32 = 4;
    const MAX_LENGTH: u32 = 150;

    const WHITE_DEPTH: u8 = 6;
    const BLACK_DEPTH: u8 = 6;

    let rng = &mut thread_rng();
    let mut cache = LruCache::new(CACHE_SIZE);

    let start_time = Instant::now();

    for _ in START_NUM..MAX_LENGTH + START_NUM {
        let mut step = |game: &mut Game, mov| {
            game.make_move(mov);

            write!(file, "{}{}", mov.get_source(), mov.get_dest()).unwrap();
            write!(
                file,
                "{} ",
                mov.get_promotion()
                    .map_or(String::new(), |x| x.to_string(Color::White))
            )
            .unwrap();

            /*if score < SCORE_DISPLAY_THRESHOLD || score > -SCORE_DISPLAY_THRESHOLD {
                print!("{{{:.2}}} ", score);
            } else if score > 0.0 {
                print!("{{inf}} ")
            } else {
                print!("{{-inf}}")
            }*/
        };

        if let Some((mov, _score)) =
            negamax_prelude(&game.current_position(), WHITE_DEPTH, rng, &mut cache)
        {
            step(&mut game, mov);
        } else {
            break;
        }

        if let Some((mov, _score)) =
            negamax_prelude(&game.current_position(), BLACK_DEPTH, rng, &mut cache)
        {
            step(&mut game, mov);
        } else {
            break;
        }
    }

    let end_time = Instant::now();

    println!();
    println!("time used: {:?}", end_time - start_time);
}
*/