#[macro_use]
extern crate crossterm;

use crossterm::cursor;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};

use std::io::stdout;

use itertools::Itertools;

use board::{Board, PushState};


fn main() {
    const SPACING: u16 = 12;

    let width = 50u16;
    let height = 25u16;

    let mut working_board = Board::new(width as usize, height as usize, 100).unwrap();

    let mut stdout = stdout();
    enable_raw_mode().unwrap();

    execute!(
        stdout, 
        Clear(ClearType::All), 
        cursor::MoveTo(0, 0),
        cursor::DisableBlinking,
    );

    print!("╔═════╦");
    for _ in 0..width - SPACING { print!("═") }
    print!("╦═════╗\n");

    print!("║ {:03} ║", working_board.mine_total);

    for _ in 0..width - SPACING { print!(" ") }

    print!("║ 000 ║\n");
    print!("╠═════╩");
    for _ in 0..width - SPACING { print!("═") }
    print!("╩═════╣\n");

    println!("{}", working_board);

    print!("╚");
    for _ in 0..width {
        print!("═");
    }
    print!("╝");

    execute!(
        stdout,
        cursor::MoveTo(1, 3),
    );

    let mut cursor_pos: (u16, u16) = (0, 0);

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
                if cursor_pos.0 < width - 1 {
                    execute!(stdout.lock(), cursor::MoveRight(1)).unwrap();
                    cursor_pos.0 += 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Left, ..
            }) | Event::Key(KeyEvent {
                code: KeyCode::Char('a'), ..
            }) => {
                if cursor_pos.0 > 0 {
                    execute!(stdout.lock(), cursor::MoveLeft(1)).unwrap();
                    cursor_pos.0 -= 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Up, ..
            }) | Event::Key(KeyEvent {
                code: KeyCode::Char('w'), ..
            }) => {
                if cursor_pos.1 > 0 {
                    execute!(stdout.lock(), cursor::MoveUp(1)).unwrap();
                    cursor_pos.1 -= 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Down, ..
            }) | Event::Key(KeyEvent {
                code: KeyCode::Char('s'), ..
            }) => {
                if cursor_pos.1 < height - 1 {
                    execute!(stdout.lock(), cursor::MoveDown(1)).unwrap();
                    cursor_pos.1 += 1;
                }
            },
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'), ..
            }) => {
                working_board.push_state(cursor_pos.0 as usize, cursor_pos.1 as usize, PushState::Uncover);
                refresh_board(&cursor_pos, &working_board, &width);
            },
            Event::Key(KeyEvent {
                code: KeyCode::Char('e'), ..
            }) => {
                working_board.push_state(cursor_pos.0 as usize, cursor_pos.1 as usize, PushState::Flag);
                refresh_board(&cursor_pos, &working_board, &width);
            },
            _ => (),
        }
    }

    disable_raw_mode().unwrap();
}

fn refresh_board(pos: &(u16, u16), working_board: &Board, width: &u16) {
    let stdout = stdout();
    let mut stdout_handle = stdout.lock();

    execute!(
        stdout_handle, 
        cursor::Hide,
        cursor::MoveTo(0, 3),
        Print(working_board),
    );

    execute!(stdout_handle, cursor::MoveTo(2, 1));
    print!("{:03}", working_board.mine_total - working_board.flag_total);

    if let Some(i) = working_board.won {
        let spacer = width / 2 - 3;

        execute!(stdout_handle, cursor::MoveTo(spacer, 1));

        print!("{}", match i {
            true if width % 2 == 0 => "YOU  WON",
            true => " YOU WON ",
            false if width % 2 == 0 => "YOU LOST",
            false => "YOU  LOST"
        });
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
        uncover_correct: usize,
    }

    impl Board {
        pub fn new(width: usize, height: usize, mine_num: usize) -> Result<Board, String> {
            let total = width * height;

            if total < mine_num {
                return Err(String::from("There cannot be more mines then there are tiles"));
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
                uncover_correct: 0,
                won: None,
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

            if self.flag_correct == self.mine_total || self.uncover_correct == self.tiles.len() - self.mine_total {
                self.end_game(true);
            }
        }

        fn set_tile_state(&mut self, x: usize, y: usize, update: State) {
            self.tiles[get_1d(x, y, self.width)].state = update;
        }

        fn get_tile(&self, x: usize, y: usize) -> Option<&Tile> {
            self.tiles.get(get_1d(x, y, self.width))
        }

        fn uncover_tile(&mut self, x: usize, y: usize) {
            let mut tile = &mut self.tiles[get_1d(x, y, self.width)];

            if tile.mine {
                self.end_game(false);
                return;
            }

            tile.state = State::Uncovered;

            if tile.mines_surrounding == 0 {
                // self.clear_zeros();
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

            let mut uncover = vec!(starting_pos);
            let mut working = uncover.clone();

            while !working.is_empty() {
                let surroundings: Vec<usize> = working.iter()
                    .map(|i| get_1d_manhattan(*i, self.width))
                    .flatten()
                    .unique()
                    .filter(|i| !uncover.contains(i))
                    .filter(|i| match self.tiles.get(*i) {
                        Some(n) if n.state == State::Covered => true,
                        _ => false,
                    })
                    .collect();

                uncover.extend(&surroundings);

                working.clear();

                for t in surroundings {
                    if self.tiles.get(t).unwrap().mines_surrounding == 0 {
                        working.push(t);
                    }
                }
            }

            for t in uncover {
                self.tiles.get_mut(t).unwrap().state = State::Uncovered;
            }
        }
    }

    impl fmt::Display for Board {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

            // let spacing = 12;

            let mut formatted = String::from("");

            // formatted.push_str("╔═════╦");
            // for _ in 0..self.width - spacing { formatted.push_str("═") }
            // formatted.push_str("╦═════╗\n");

            // formatted.push_str(&format!("║ {:03} ║", self.mine_total - self.flag_total)[..]);

            // if let Some(i) = self.won {
            //     let spacer = (self.width - spacing - 8) / 2;

            //     for _ in 0..spacer { formatted.push_str(" ") }

            //     formatted.push_str(match i {
            //         true if self.width % 2 == 0 => "YOU  WON",
            //         true => " YOU WON ",
            //         false if self.width % 2 == 0 => "YOU LOST",
            //         false => "YOU  LOST"
            //     });

            //     for _ in 0..spacer { formatted.push_str(" ") }

            // } else {
            //     for _ in 0..self.width - spacing { formatted.push_str(" ") }
            // }

            // formatted.push_str("║ 000 ║\n");
            // formatted.push_str("╠═════╩");
            // for _ in 0..self.width - spacing { formatted.push_str("═") }
            // formatted.push_str("╩═════╣\n║");

            formatted.push_str("║");

            for (count, tile) in self.tiles.iter().enumerate() {
                formatted.push_str(&tile.to_string());

                if count == self.tiles.len() - 1 {
                    formatted.push_str("║")
                } else if count % self.width == self.width - 1 {
                    formatted.push_str("║\n║");
                }
            }

            // formatted.push_str("╚");
            // for _ in 0..self.width {
            //     formatted.push_str("═");
            // }
            // formatted.push_str("╝");

            write!(f, "{}", formatted)
        }
    }

    mod tests {
        use super::*;

        #[test]
        fn board_clear() {
            let mut test_board = Board::new(25, 25, 0).unwrap();
            test_board.push_state(0, 0, PushState::Uncover);

            assert!(test_board.tiles.iter()
                .fold(true, |t, i| t && i.state == State::Uncovered)
            );
        }
    }
}