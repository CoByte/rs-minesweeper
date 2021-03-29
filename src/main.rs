#![allow(warnings)]

#[macro_use]
extern crate crossterm;

#[macro_use]
extern crate clap;

use crossterm::cursor;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, DisableLineWrap, EnableLineWrap};

use clap::{App, Arg};

use std::io::stdout;
use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use std::cmp;

use itertools::Itertools;

use board::{Board, PushState};


arg_enum! {
    #[derive(Debug)]
    enum Difficulty {
        Beginner,
        Intermediate,
        Expert,
    }
}

impl Difficulty {
    fn value(&self) -> f32 {
        match *self {
            Difficulty::Beginner => 0.1235,
            Difficulty::Intermediate => 0.1563,
            Difficulty::Expert => 0.2062,
        }
    }
}


fn main() {

    let matches = App::new("rs-minesweeper")
        .arg(
            Arg::with_name("width")
                .help("Sets the width of the board. The minimum is 22, and the maximum is 2 less then your terminal width")
                .long("width")
                .short("w")
                .value_name("WIDTH")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("height")
                .help("Sets the height of the board. The minimum is 1, and the maximum is 5 less then your terminal height")
                .long("height")
                .short("h")
                .value_name("HEIGHT")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("mine_num")
                .help("Sets the number of mines on the board. Must be one less then the total number of tiles")
                .long("mines")
                .short("m")
                .value_name("MINES")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("max_width")
                .help("Sets the width to its maximum. Not compatible with -w [WIDTH]")
                .long("max-width")
                .conflicts_with("width")
        )
        .arg(
            Arg::with_name("max_height")
                .help("Sets the width to its maximum. Not compatible with -h [HEIGHT]")
                .long("max-height")
                .conflicts_with("height")
        )
        .arg(
            Arg::with_name("difficulty")
                .help("Creates a board of either beginner, intermediate, or expert difficulty")
                .short("d")
                .long("difficulty")
                .conflicts_with_all(&["height", "width", "width_max", "height_max"])
                .value_name("DIFFICULTY")
                .takes_value(true)
                .possible_values(&Difficulty::variants())
                .case_insensitive(true)
        )
        .arg(
            Arg::with_name("smart_difficulty")
                .help("Sets the number of mines based on preset difficulty ratios")
                .short("s")
                .long("smart-difficulty")
                .conflicts_with_all(&["mine_num", "difficulty"])
                .value_name("DIFFICULTY")
                .takes_value(true)
                .possible_values(&Difficulty::variants())
                .case_insensitive(true)
        )
        .get_matches();

    const SPACING: u16 = 12;

    let mut width = value_t!(matches, "width", u16).unwrap_or(22);
    let mut height = value_t!(matches, "height", u16).unwrap_or(12);
    let mut mine_num = value_t!(matches, "mine_num", u16).unwrap_or(41);

    let size = size().unwrap();
    let size = (size.0 - 2, size.1 - 5);

    if width > size.0 { 
        println!("error: width cannot be larger then the terminal width - 2");
        return; 
    }

    if height > size.1 { 
        println!("error: height cannot be larger then the terminal height - 5");
        return; 
    }

    if mine_num >= width * height {
        println!("error: number of mines cannot be equal to or larger then the total number of tiles");
        return;
    }

    if matches.is_present("max_width") {
        width = size.0;
    }

    if matches.is_present("max_height") {
        height = size.1;
    }

    if let Ok(i) = value_t!(matches, "difficulty", Difficulty) {
        match i {
            Difficulty::Beginner => {
                width = 22;
                height = 4;
                mine_num = 11;
            },
            Difficulty::Intermediate => {
                width = 22;
                height = 12;
                mine_num = 41;
            }
            Difficulty::Expert => {
                width = 22;
                height = 22;
                mine_num = 100;
            }
        }
    }

    if let Ok(i) = value_t!(matches, "smart_difficulty", Difficulty) {
        mine_num = ((width * height) as f32 * Difficulty::value(&i)) as u16;
    }

    let mut working_board = Board::new(width as usize, height as usize, mine_num as usize).unwrap();

    let mut stdout = stdout();
    enable_raw_mode().unwrap();

    execute!(
        stdout, 
        Clear(ClearType::All), 
        cursor::MoveTo(0, 0),
        cursor::DisableBlinking,
        DisableLineWrap,
    );

    print!("╔═════╦");
    for _ in 0..width - SPACING { print!("═") }
    print!("╦═════╗\r\n");

    print!("║ {:03} ║", cmp::min(working_board.mine_total, 999));

    for _ in 0..width - SPACING { print!(" ") }

    print!("║ 000 ║\r\n");
    print!("╠═════╩");
    for _ in 0..width - SPACING { print!("═") }
    print!("╩═════╣\r\n");

    print!("{}\r\n", working_board);

    print!("╚");
    for _ in 0..width {
        print!("═");
    }
    print!("╝");

    execute!(
        stdout,
        cursor::MoveTo(1, 3),
    );

    let cursor_pos = Arc::new(Mutex::new((0u16, 0u16)));

    let (main_tx, clock_rx) = mpsc::channel::<bool>();
    launch_clock(Arc::clone(&cursor_pos), width.clone(), clock_rx);

    loop {  
        match read().unwrap() {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            }) | Event::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) => {
                execute!(stdout.lock(), cursor::MoveTo(0, 0), Clear(ClearType::All));
                break
            },
            Event::Key(KeyEvent {
                code: KeyCode::Right, ..
            }) | Event::Key(KeyEvent {
                code: KeyCode::Char('d'), ..
            }) => {
                let mut pos = cursor_pos.lock().unwrap();

                if pos.0 < width - 1 {
                    execute!(stdout.lock(), cursor::MoveRight(1)).unwrap();
                    pos.0 += 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Left, ..
            }) | Event::Key(KeyEvent {
                code: KeyCode::Char('a'), ..
            }) => {
                let mut pos = cursor_pos.lock().unwrap();

                if pos.0 > 0 {
                    execute!(stdout.lock(), cursor::MoveLeft(1)).unwrap();
                    pos.0 -= 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Up, ..
            }) | Event::Key(KeyEvent {
                code: KeyCode::Char('w'), ..
            }) => {
                let mut pos = cursor_pos.lock().unwrap();

                if pos.1 > 0 {
                    execute!(stdout.lock(), cursor::MoveUp(1)).unwrap();
                    pos.1 -= 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Down, ..
            }) | Event::Key(KeyEvent {
                code: KeyCode::Char('s'), ..
            }) => {
                let mut pos = cursor_pos.lock().unwrap();

                if pos.1 < height - 1 {
                    execute!(stdout.lock(), cursor::MoveDown(1)).unwrap();
                    pos.1 += 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'), ..
            }) => {
                let pos = cursor_pos.lock().unwrap();

                working_board.push_state(pos.0 as usize, pos.1 as usize, PushState::Uncover);
                refresh_board(&pos, &working_board, &width, &main_tx);

                if working_board.won.is_some() { 
                    execute!(stdout.lock(), cursor::MoveTo(0, height + 4));
                    break 
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Char('e'), ..
            }) => {
                let pos = cursor_pos.lock().unwrap();

                working_board.push_state(pos.0 as usize, pos.1 as usize, PushState::Flag);
                refresh_board(&pos, &working_board, &width, &main_tx);

                if working_board.won.is_some() { 
                    execute!(stdout.lock(), cursor::MoveTo(0, height + 4));
                    break 
                }
            },
            _ => (),
        }
    }

    execute!(stdout, EnableLineWrap);
    disable_raw_mode().unwrap();
}

fn launch_clock(cursor_pos: Arc<Mutex<(u16, u16)>>, width: u16, rx: mpsc::Receiver<bool>) {
    let stdout = stdout();
    let mut time = 0;

    thread::spawn(move || { 
        if !rx.recv().unwrap() {
            return;
        }

        for _ in 0..1000 {
            thread::sleep(Duration::from_secs(1));

            if Ok(false) == rx.try_recv() {
                return;
            }

            let mut stdout_handle = stdout.lock();
            let pos = cursor_pos.lock().unwrap();
            time += 1;

            execute!(
                stdout_handle,
                cursor::MoveTo(width - 3, 1),
                Print(&format!("{:03}", time)[..]),
                cursor::MoveTo(pos.0 + 1, pos.1 + 3),
            );
        }
    });
}

fn refresh_board(pos: &(u16, u16), working_board: &Board, width: &u16, tx: &mpsc::Sender<bool>) {
    let stdout = stdout();
    let mut stdout_handle = stdout.lock();

    execute!(
        stdout_handle, 
        cursor::Hide,
        cursor::MoveTo(0, 3),
        Print(working_board),
    );

    execute!(
        stdout_handle, 
        cursor::MoveTo(2, 1),
        Print(&format!("{:03}", cmp::min(working_board.mine_total - working_board.flag_total, 999))[..])
    );

    if let Some(i) = working_board.won {
        let spacer = width / 2 - 3;

        execute!(
            stdout_handle, 
            cursor::MoveTo(spacer, 1),
            Print(match i {
                true if width % 2 == 0 => "YOU  WON",
                true => " YOU WON ",
                false if width % 2 == 0 => "YOU LOST",
                false => "YOU  LOST"
            }),
        );

        let _ = tx.send(false);
    } else {
        let _ = tx.send(true);
    }

    execute!(
        stdout_handle,
        cursor::MoveTo(pos.0 + 1, pos.1 + 3),
        cursor::Show,
    );
}

mod board {
    use rand::thread_rng;
    use rand::seq::SliceRandom;
    use std::fmt;

    use crossterm::style::Colorize;

    use super::*;

    fn get_manhattan() -> Vec<(i32, i32)> {
        vec![
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1)
        ]
    }

    fn get_2d(i: usize, width: usize) -> (usize, usize) {
        (i % width, i / width)
    }

    fn get_1d(x: usize, y: usize, width: usize) -> usize {
        y * width + x
    }

    fn get_1d_manhattan(i: usize, width: usize) -> Vec<usize> {
        let (x, y) = get_2d(i, width);

        get_manhattan().iter()
            .map(|i| (i.0 + x as i32, i.1 + y as i32))
            .filter_map(|i| match i {
                (x, y) if width as i32 > x && x >= 0 && y >= 0 => Some(
                    get_1d(x as usize, y as usize, width)
                ),
                _ => None,
            }).collect()
    }

    #[derive(PartialEq, Hash, Debug, Clone)]
    enum State {
        Uncovered,
        Covered,
        Flagged,
        FlagRevealed,
    }

    pub enum PushState {
        Uncover,
        Flag,
    }

    #[derive(PartialEq, Hash, Debug, Clone)]
    pub struct Tile {
        state: State,
        mine: bool,
        mines_surrounding: usize,
    }

    impl fmt::Display for Tile {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", match self.state {
                State::Covered => String::from("░"),
                State::Uncovered if self.mine => String::from("Ø").red().to_string(),
                State::Uncovered if self.mines_surrounding > 0 => self.mines_surrounding.to_string(),
                State::Uncovered => String::from(" "),
                State::Flagged => String::from("Þ").green().to_string(),
                State::FlagRevealed if self.mine => String::from("Þ").green().to_string(),
                State::FlagRevealed => String::from("Þ").yellow().to_string(),
            })
        }
    }

    impl Tile {
        fn new(mine: &bool, mines_surrounding: &usize) -> Tile {
            Tile {
                state: State::Covered,
                mine: *mine,
                mines_surrounding: *mines_surrounding
            }
        }
    }

    #[derive(PartialEq, Debug)]
    pub struct Board {
        pub tiles: Vec<Tile>,
        pub won: Option<bool>,
        width: usize,
        pub mine_total: usize,
        pub flag_total: usize,
        flag_correct: usize,
        first_uncover: bool,
    }

    impl Board {
        pub fn new(width: usize, height: usize, mine_num: usize) -> Result<Board, String> {
            let total = width * height;

            if total < mine_num {
                return Err(String::from("There cannot be more mines then there are tiles"));

            } else if total == mine_num {
                return Err(String::from("At least one tile must be safe"));
            }

            let mut mine_values = vec![true; mine_num];
            mine_values.extend(vec![false; total - mine_num]);
            mine_values.shuffle(&mut thread_rng());

            let mine_totals: Vec<usize> = mine_values.iter().enumerate()
                .map(|i| get_1d_manhattan(i.0, width))
                .map(|i| {
                    i.iter()
                        .filter_map(|n| mine_values.get(*n as usize))
                        .fold(0, |t, n| t + *n as usize)
                }).collect();

            let tile_data = mine_values.iter().zip(mine_totals.iter());
            let tiles: Vec<_> = tile_data.map(|i| Tile::new(i.0, i.1)).collect();
                        
            Ok(Board {
                tiles: tiles,
                width: width,
                mine_total: mine_num,
                flag_total: 0,
                flag_correct: 0,
                won: None,
                first_uncover: true,
            })
        }
        
        pub fn push_state(&mut self, x: usize, y: usize, update: PushState) {
            if self.won.is_some() {
                return
            }

            let old_tile = self.get_tile(x, y).unwrap();

            match (&old_tile.state, update) {
                (State::Covered, PushState::Uncover) => {
                    self.uncover_tile(x, y);
                },
                (State::Uncovered, _) => {
                    let manhattan_tile_coords = get_1d_manhattan(
                        get_1d(x, y, self.width), self.width);

                    let flags_surrounding = manhattan_tile_coords.iter()
                        .filter_map(|i| self.tiles.get(*i))
                        .fold(0, |t, i| t + (i.state == State::Flagged) as usize);
                    
                    if flags_surrounding == old_tile.mines_surrounding {
                        for coord in manhattan_tile_coords {
                            if let Some(t) = self.tiles.get(coord) {
                                if t.state == State::Covered {
                                    let coords = get_2d(coord, self.width);
                                    self.uncover_tile(coords.0, coords.1);
                                }
                            }
                        }
                    }
                },
                (State::Flagged, PushState::Flag) => {
                    self.flag_total -= 1;
                    self.set_tile_state(x, y, State::Covered);

                    if self.get_tile(x, y).unwrap().mine {
                        self.flag_correct -= 1;
                    }
                },
                (State::Covered, PushState::Flag) => {
                    if self.flag_total < self.mine_total {
                        self.flag_total += 1;
                        self.set_tile_state(x, y, State::Flagged);

                        if self.get_tile(x, y).unwrap().mine {
                            self.flag_correct += 1;
                        }
                    }
                },
                _ => (),
            };

            let uncover_correct = self.tiles.iter()
                .filter(|i| i.state == State::Uncovered && !i.mine)
                .collect::<Vec<_>>().len();

            if self.won.is_none() {
                if self.flag_correct == self.mine_total || uncover_correct == self.tiles.len() - self.mine_total {
                    self.end_game(true);
                }
            }
        }

        fn set_tile_state(&mut self, x: usize, y: usize, update: State) {
            self.tiles[get_1d(x, y, self.width)].state = update;
        }

        fn get_tile(&self, x: usize, y: usize) -> Option<&Tile> {
            self.tiles.get(get_1d(x, y, self.width))
        }

        fn uncover_tile(&mut self, x: usize, y: usize) {
            let tile_pos = get_1d(x, y, self.width);
            let mut tile = &mut self.tiles[tile_pos];

            if tile.mine && self.first_uncover {
                tile.mine = false;

                for s in get_1d_manhattan(tile_pos, self.width) {
                    if let Some(i) = self.tiles.get_mut(s) {
                        i.mines_surrounding -= 1;
                    }
                }
                
                let mut possible_replacements: Vec<_> = self.tiles.iter().enumerate()
                    .filter(|i| !i.1.mine)
                    .map(|i| i.0)
                    .collect();
                possible_replacements.shuffle(&mut thread_rng());
                let replacement = possible_replacements[0];

                let mut swap_tile = &mut self.tiles[replacement];
                swap_tile.mine = true;

                for s in get_1d_manhattan(replacement, self.width) {
                    if let Some(i) = self.tiles.get_mut(s) {
                        i.mines_surrounding += 1;
                    }
                }

            } else if tile.mine {
                self.end_game(false);
                return;
            }

            self.first_uncover = false;

            let mut tile = &mut self.tiles[tile_pos];

            tile.state = State::Uncovered;

            if tile.mines_surrounding == 0 {
                self.clear_zeros((x, y));
            }
        }

        fn end_game(&mut self, won: bool) {
            self.won = Some(won);

            for t in &mut self.tiles {
                t.state = match t.state {
                    State::Flagged => State::FlagRevealed,
                    _ => State::Uncovered,
                }
            }
        }

        fn clear_zeros(&mut self, starting_pos: (usize, usize)) {
            let starting_pos = get_1d(starting_pos.0, starting_pos.1, self.width);

            let mut working = vec!(starting_pos);

            while !working.is_empty() {
                for t in &working {
                    self.tiles.get_mut(*t).unwrap().state = State::Uncovered;
                }

                let surroundings: Vec<usize> = working.iter()
                    .map(|i| get_1d_manhattan(*i, self.width))
                    .flatten()
                    .unique()
                    .filter(|i| match self.tiles.get(*i) {
                        Some(n) if n.state == State::Covered => true,
                        _ => false,
                    })
                    .collect();

                working.clear();

                for t in surroundings {
                    if self.tiles.get(t).unwrap().mines_surrounding == 0 {
                        working.push(t);
                    } else {
                        self.tiles.get_mut(t).unwrap().state = State::Uncovered;
                    }
                }
            }
        }
    }

    impl fmt::Display for Board {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

            let mut formatted = String::from("║");

            for (count, tile) in self.tiles.iter().enumerate() {
                formatted.push_str(&tile.to_string());

                if count == self.tiles.len() - 1 {
                    formatted.push_str("║")
                } else if count % self.width == self.width - 1 {
                    formatted.push_str("║\r\n║");
                }
            }

            write!(f, "{}", formatted)
        }
    }

    mod tests {
        use super::*;

        #[test]
        fn board_clear() {
            let mut test_board = Board::new(188, 66, 0).unwrap();
            test_board.push_state(0, 0, PushState::Uncover);

            assert!(test_board.tiles.iter()
                .fold(true, |t, i| t && i.state == State::Uncovered)
            );
        }
    }
}