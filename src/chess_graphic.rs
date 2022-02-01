use crate::chess_minmax::{negamax_prelude, negamax_prelude_2nd, BoardHash, TranspositionItem};

use chess::{
    Action, BitBoard, Board, BoardStatus, ChessMove, Color, File, Game, Piece, Rank, Square,
};
use itertools::Itertools;
use lru::LruCache;
use piston_window::*;
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};
use seahash::SeaHasher;
use std::hash::{BuildHasher, BuildHasherDefault};
use std::io::{stdin, stdout, Write};
use std::str::FromStr;
use std::collections::HashSet;

pub mod colors;

const NUM_FILE: usize = 8;
const NUM_RANK: usize = 8;

struct ChessTexture {
    white_pawn: G2dTexture,
    black_pawn: G2dTexture,
    white_king: G2dTexture,
    black_king: G2dTexture,
    white_rook: G2dTexture,
    black_rook: G2dTexture,
    white_knight: G2dTexture,
    black_knight: G2dTexture,
    white_queen: G2dTexture,
    black_queen: G2dTexture,
    white_bishop: G2dTexture,
    black_bishop: G2dTexture,
}

impl ChessTexture {
    const IMG_SIZE: u32 = 90;

    pub fn new(context: &mut G2dTextureContext) -> Self {
        let empty_setting = TextureSettings::new();

        let mut load_image =
            |path| G2dTexture::from_path(context, path, Flip::None, &empty_setting).unwrap();

        ChessTexture {
            white_pawn: load_image("asset/white_pawn.png"),
            black_pawn: load_image("asset/black_pawn.png"),
            white_king: load_image("asset/white_king.png"),
            black_king: load_image("asset/black_king.png"),
            white_rook: load_image("asset/white_rook.png"),
            black_rook: load_image("asset/black_rook.png"),
            white_knight: load_image("asset/white_knight.png"),
            black_knight: load_image("asset/black_knight.png"),
            white_queen: load_image("asset/white_queen.png"),
            black_queen: load_image("asset/black_queen.png"),
            white_bishop: load_image("asset/white_bishop.png"),
            black_bishop: load_image("asset/black_bishop.png"),
        }
    }
}

pub struct ChessGraphic {
    base_game: Game,
    chess_game: Game,
    selecting: Option<Square>,
    mouse_x: f64,
    mouse_y: f64,
    draw_size: [u32; 2],
    rng: ThreadRng,
    cache: LruCache<BoardHash, TranspositionItem, BuildHasherDefault<SeaHasher>>,
    dirty: bool,
    textures: ChessTexture,
    depth: u8,
    enable_ai: bool,
    display_swap_side: bool,
}

impl ChessGraphic {
    pub fn new(texture_context: &mut G2dTextureContext) -> Self {
        Self::from_game(Game::new(), texture_context)
    }

    pub fn from_str(
        fen: &str,
        texture_context: &mut G2dTextureContext,
    ) -> Result<Self, <Game as FromStr>::Err> {
        Ok(Self::from_game(Game::from_str(fen)?, texture_context))
    }

    fn print_control_message() {
        println!("SEMICOLON (;): make AI play");
        println!("BACKSLASH (/): make AI play 2nd best move");
        println!("Z: Undo move (if possible)");
        println!("A: toggle AI");
        println!("F: print FEN");
        println!("I: Input FEN");
        println!("H: print PNG history");
        println!("R: Reset Game");
        println!("S: Swap Side");
        println!("RIGHT: increase AI depth");
        println!("LEFT: decrease AI depth");
    }

    pub fn from_game(game: Game, texture_context: &mut G2dTextureContext) -> Self {
        const DEFAULT_DEPTH: u8 = 4;
        println!("Game initialized with Depth {} AI\n", DEFAULT_DEPTH);

        Self::print_control_message();
        println!();

        ChessGraphic {
            base_game: game.clone(),
            chess_game: game,
            selecting: None,
            mouse_x: Default::default(),
            mouse_y: Default::default(),
            draw_size: Default::default(),
            rng: thread_rng(),
            cache: LruCache::with_hasher(crate::CACHE_SIZE, Default::default()),
            dirty: true,
            textures: ChessTexture::new(texture_context),
            depth: DEFAULT_DEPTH,
            enable_ai: true,
            display_swap_side: false,
        }
    }

    pub fn reset(&mut self) {
        self.chess_game = Game::new();
        self.base_game = self.chess_game.clone();
        self.selecting = None;

        println!("Clearing Cache...");
        self.cache.clear();
        println!("Done");

        println!();
        Self::print_control_message();
        println!();
    }

    pub fn input_fen(&mut self) {
        print!("Input FEN: ");
        stdout().flush().unwrap();

        let mut fen = String::with_capacity(70 /* normal length of FEN string */);
        stdin().read_line(&mut fen).unwrap();

        debug_assert!(fen.len() <= 70);

        match Game::from_str(fen.as_str()) {
            Err(e) => println!("{}", e),
            Ok(game) => {
                self.reset();
                self.chess_game = game;
                self.base_game = self.chess_game.clone();
            }
        }
    }

    pub fn png_history(&mut self) {
        // let s = String::new();
        for act in self.chess_game.actions() {
            if let Action::MakeMove(mov) = act {
                print!("{} ", Self::format_move(mov));
            }
        }
        println!();
    }

    // DRAW
    pub fn draw(&mut self, c: Context, g: &mut G2d) {
        if self.dirty {
            self.redraw(c, g);
            self.dirty = true;
        }
    }

    fn redraw(&self, c: Context, g: &mut G2d) {
        Self::draw_grid(c, g, 8, 8);
        if let Some(&last_mov) = self.chess_game.actions().iter().rev().find_map(|act| {
            if let Action::MakeMove(mov) = act {
                Some(mov)
            } else {
                None
            }
        }) {
            Self::draw_last_move(c, g, last_mov, self.display_swap_side);
        }
        Self::draw_pieces(
            c,
            g,
            &self.chess_game.current_position(),
            &self.textures,
            self.display_swap_side,
        );

        if let Some(square) = self.selecting {
            Self::draw_selecting(c, g, square, self.display_swap_side);
        }
    }

    fn draw_grid(c: Context, g: &mut G2d, n_width: u32, n_height: u32) {
        let [w, h] = c.viewport.unwrap().window_size;

        let dw = w as u32 / n_width;
        let dh = h as u32 / n_height;

        for (i, j) in (0..n_width).cartesian_product(0..n_height) {
            let x0 = f64::from(dw * i);
            let x1 = f64::from(dw * (i + 1));
            let y0 = f64::from(dh * j);
            let y1 = f64::from(dh * (j + 1));
            let grid_rect = rectangle::rectangle_by_corners(x0, y0, x1, y1);

            let grid_color = match (i + j) % 2 {
                0 => colors::GRID_COLOR_1,
                1 => colors::GRID_COLOR_2,
                _ => unreachable!(),
            };

            rectangle(grid_color, grid_rect, c.transform, g);
        }
    }

    fn draw_last_move(c: Context, g: &mut G2d, last_mov: ChessMove, swap: bool) {
        let source_rect = Self::square_to_rect(&last_mov.get_source(), &c.viewport.unwrap(), swap);
        rectangle(colors::GRID_COLOR_MOVED, source_rect, c.transform, g);
        let destination_rect =
            Self::square_to_rect(&last_mov.get_dest(), &c.viewport.unwrap(), swap);
        rectangle(colors::GRID_COLOR_MOVED, destination_rect, c.transform, g);
    }

    fn draw_pieces(c: Context, g: &mut G2d, board: &Board, textures: &ChessTexture, swap: bool) {
        let vp_ref = &c.viewport.unwrap();

        let img_size = ChessTexture::IMG_SIZE as f64;
        let [view_width, view_height] = vp_ref.window_size;
        let [view_width, view_height] = [view_width as f64, view_height as f64];
        let grid_width = view_width / NUM_FILE as f64; // grid width
        let grid_height = view_height / NUM_RANK as f64; // grid height
        let sx = grid_width / img_size;
        let sy = grid_height / img_size;

        let white = board.color_combined(Color::White);
        let black = board.color_combined(Color::Black);

        let pawn = board.pieces(Piece::Pawn);
        let bishop = board.pieces(Piece::Bishop);
        let rook = board.pieces(Piece::Rook);
        let knight = board.pieces(Piece::Knight);
        let queen = board.pieces(Piece::Queen);
        let king = board.pieces(Piece::King);

        let mut draw_piece = |bitboard: BitBoard, texture: &G2dTexture| {
            bitboard.for_each(|square| {
                let [x0, y0, _, _] = Self::square_to_rect(&square, vp_ref, swap);
                image(texture, c.trans(x0, y0).scale(sx, sy).transform, g);
            })
        };

        draw_piece(white & pawn, &textures.white_pawn);
        draw_piece(white & knight, &textures.white_knight);
        draw_piece(white & bishop, &textures.white_bishop);
        draw_piece(white & rook, &textures.white_rook);
        draw_piece(white & king, &textures.white_king);
        draw_piece(white & queen, &textures.white_queen);

        draw_piece(black & pawn, &textures.black_pawn);
        draw_piece(black & knight, &textures.black_knight);
        draw_piece(black & bishop, &textures.black_bishop);
        draw_piece(black & rook, &textures.black_rook);
        draw_piece(black & king, &textures.black_king);
        draw_piece(black & queen, &textures.black_queen);
    }

    fn draw_selecting(c: Context, g: &mut G2d, square: Square, swap: bool) {
        let draw_rect = Self::square_to_rect(&square, &c.viewport.unwrap(), swap);
        let marking_rect = rectangle::margin(draw_rect, 0.5);

        ellipse(colors::COLOR_SELECTED, marking_rect, c.transform, g);
    }

    // INPUT HANDLING
    pub fn button_input(&mut self, button: &Button) {
        match button {
            Button::Keyboard(key) => self.keyboard_input(*key),
            Button::Mouse(mouse) => self.mouse_input(*mouse),
            Button::Controller(_) => {}
            Button::Hat(_) => {}
        }
    }

    fn mouse_input(&mut self, mouse: MouseButton) {
        if mouse != MouseButton::Left {
            return;
        }

        self.mark_dirty();
        let clicking_square = Self::pos_to_square(
            self.draw_size,
            self.mouse_x,
            self.mouse_y,
            self.display_swap_side,
        );

        match self.selecting {
            // no square previously select
            None => {
                self.selecting = Some(clicking_square);
            }
            // predicate "there exist square for which the user previously select" is true
            Some(select_square) => {
                // handle promotion
                let is_selecting_pawn = || {
                    self.chess_game
                        .current_position()
                        .piece_on(select_square)
                        .map_or(false, |x| x == Piece::Pawn)
                };
                let is_clicking_at_promotable_square = || {
                    let promotable_rank = match self.chess_game.side_to_move() {
                        Color::White => Rank::Eighth,
                        Color::Black => Rank::First,
                    };
                    clicking_square.get_rank() == promotable_rank
                };

                let promotion = if is_clicking_at_promotable_square() && is_selecting_pawn() {
                    // TODO: user select promotion?
                    Some(Piece::Queen)
                } else {
                    None
                };

                // generate user's move
                let mov = ChessMove::new(select_square, clicking_square, promotion);

                // check legality
                if self.chess_game.current_position().legal(mov) {
                    // move is legal
                    self.make_move_msg(mov); // make that legal move
                    self.selecting = None; // deselect the pieces

                    if self.enable_ai {
                        self.ai_play(false);
                    }
                } else {
                    // move is illegal
                    self.selecting = None; // deselect the pieces
                }
            }
        }
    }

    fn keyboard_input(&mut self, key: Key) {
        match key {
            Key::F => println!("{}", self.chess_game.current_position()),
            Key::Semicolon => self.ai_play(false),
            Key::Backslash => self.ai_play(true),
            Key::Z => self.undo(),
            Key::Right | Key::Plus | Key::NumPadPlus => {
                self.depth += 1;
                println!("AI: Set Depth={}", self.depth)
            }
            Key::Left | Key::Minus | Key::NumPadMinus => {
                self.depth = self.depth.saturating_sub(1);
                println!("AI: Set Depth={}", self.depth)
            }
            Key::H => self.png_history(),
            Key::A => {
                if self.enable_ai {
                    println!("Disable AI");
                    self.enable_ai = false;
                } else {
                    println!("Enable AI");
                    self.enable_ai = true;
                }
            }
            Key::S => {
                self.display_swap_side = !self.display_swap_side;
                self.mark_dirty();
            }
            Key::R => self.reset(),
            Key::I => self.input_fen(),
            _ => {}
        }
    }

    pub fn on_mouse_position(&mut self, mouse_pos: [f64; 2]) {
        self.mouse_x = mouse_pos[0];
        self.mouse_y = mouse_pos[1];
    }

    pub fn on_resize(&mut self, resize_args: ResizeArgs) {
        let [w, h] = resize_args.window_size;
        self.draw_size = [w as _, h as _];
    }

    fn undo(&mut self) {
        if let Some((last_act, prev_acts)) = self.chess_game.actions().split_last() {
            println!("Undo Success! ({} undo left)", prev_acts.len());

            let mut game = self.base_game.clone();

            prev_acts
                .iter()
                .filter_map(|act| {
                    if let Action::MakeMove(mov) = act {
                        Some(*mov)
                    } else {
                        None
                    }
                })
                .for_each(|mov| {
                    game.make_move(mov);
                });

            self.chess_game = game;
        } else {
            println!("Undo queue is empty");
        }
    }

    // AI BIND
    fn ai_play(&mut self, play_2nd_best: bool) {
        if !self.enable_ai {
            println!("AI: AI not enable");
            return;
        }

        let ai_result = (if play_2nd_best {
            Self::run_ai_2nd
        } else {
            Self::run_ai
        })(
            &self.chess_game.current_position(),
            &mut self.rng,
            self.depth,
            &mut self.cache,
            &Self::get_potential_repetition(&self.chess_game, &self.base_game)
        );

        if let Some((ai_move, expect_score)) = ai_result {
            println!(
                "AI ({:?}): Expected Advantage: {:.2} pawn",
                self.chess_game.current_position().side_to_move(),
                expect_score as f32 / 100.0
            );
            self.make_move_msg(ai_move);
        } else {
            println!("AI: Game Ended");
        }
    }

    // HELPER
    fn make_move(&mut self, mov: ChessMove) -> Result<bool, String> {
        match self.chess_game.current_position().status() {
            BoardStatus::Ongoing => {
                let move_result = self.chess_game.make_move(mov);
                Ok(move_result)
            }
            BoardStatus::Stalemate => Err("Stalemated".to_string()),
            BoardStatus::Checkmate => Err(format!(
                "{:?} Checkmated",
                self.chess_game.current_position().side_to_move()
            )),
        }
    }

    fn make_move_msg(&mut self, mov: ChessMove) -> bool {
        match self.make_move(mov) {
            Err(msg) => {
                println!("Error: {}", msg);
                false
            }
            Ok(val) => val,
        }
    }

    fn run_ai<K: BuildHasher>(
        board: &Board,
        rng: &mut impl Rng,
        depth: u8,
        cache: &mut LruCache<BoardHash, TranspositionItem, K>,
        repetition: &HashSet<BoardHash>
    ) -> Option<(ChessMove, i16)> {
        negamax_prelude(board, depth, rng, cache, repetition)
    }

    fn run_ai_2nd<K: BuildHasher>(
        board: &Board,
        rng: &mut impl Rng,
        depth: u8,
        cache: &mut LruCache<BoardHash, TranspositionItem, K>,
        repetition: &HashSet<BoardHash>
    ) -> Option<(ChessMove, i16)> {
        negamax_prelude_2nd(board, depth, rng, cache, repetition)[1]
    }

    fn get_potential_repetition(game: &Game, base_game: &Game) -> HashSet<BoardHash> {
        let mut occured = HashSet::with_capacity(game.actions().len());
        let mut repeated = HashSet::new();

        let mut board = base_game.current_position();

        game.actions().iter()
            .filter_map(|act| if let Action::MakeMove(mov) = act { Some(*mov) } else { None})
            .enumerate()
            .for_each(|(i, mov)| {
                let hash = BoardHash::new(&board);
                if !occured.insert(hash) {
                    repeated.insert(hash);
                }

                board = board.make_move_new(mov);
            });
        repeated
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn square_to_rect(square: &Square, viewport: &Viewport, swap: bool) -> [f64; 4] {
        let (rank, file) = (square.get_rank(), square.get_file());
        let r = {
            let r = rank.to_index() as f64;
            if swap {
                (NUM_RANK - 1) as f64 - r
            } else {
                r
            }
        };
        let f = {
            let f = file.to_index() as f64;
            if swap {
                (NUM_FILE - 1) as f64 - f
            } else {
                f
            }
        };

        let w = f64::from(viewport.window_size[0]);
        let h = f64::from(viewport.window_size[1]);

        let x0 = (w * f) / NUM_FILE as f64;
        let y0 = h - ((h * r) / NUM_RANK as f64);
        let x1 = w * (f + 1.0) / NUM_FILE as f64;
        let y1 = h - (h * (r + 1.0) / NUM_RANK as f64);

        rectangle::rectangle_by_corners(x0, y0, x1, y1)
    }

    fn pos_to_square(draw_size: [u32; 2], x: f64, y: f64, swap: bool) -> Square {
        let [w, h] = draw_size;

        let rel_x = x / f64::from(w);
        let rel_y = y / f64::from(h);

        let file = File::from_index({
            let file_pos = NUM_FILE as f64 * rel_x;
            if swap {
                NUM_FILE as f64 - file_pos
            } else {
                file_pos
            }
        } as usize);
        let rank = Rank::from_index({
            let inv_rank_pos = NUM_RANK as f64 * rel_y;
            if swap {
                inv_rank_pos
            } else {
                NUM_RANK as f64 - inv_rank_pos
            }
        } as usize);

        Square::make_square(rank, file)
    }

    fn format_move(mov: &ChessMove) -> String {
        let mut out = format!("{}{}", mov.get_source(), mov.get_dest());

        if let Some(promo) = mov.get_promotion() {
            out.push_str(&promo.to_string(Color::White))
        }

        out
    }
}
