use std::{
    collections::VecDeque, io, io::Stdout, io::Write, thread, time::Duration,
};
use termion::{
    event::Key, input::TermRead, raw::IntoRawMode, raw::RawTerminal,
};

/*
Two threads:
1. updates the state of the game and the screen
2. captures user commands
 */

type Cell = u8;
type CellPos = (usize, usize);
type Grid = Vec<Vec<Cell>>;
type IsGameValid = bool;

#[derive(Copy, Clone)]
enum SnakeDirection {
    Up,
    Right,
    Down,
    Left,
}

struct GameState {
    grid: Grid,
    // The sleep time between game state updates.
    timing: Duration,
    last_direction: SnakeDirection,
    // Positions of the head and tail in the grid.
    head: CellPos,
    tail: CellPos,
    // The head queues its positions, the tail pop positions. We can use this
    // to calculate the next tail position.
    head_directions: VecDeque<SnakeDirection>,
}

const SNAKE: Cell = 1;
const EMPTY: Cell = 0;
const FOOD: Cell = 2;

fn grid_size(grid: &Grid) -> (usize, usize) {
    (grid.len(), grid[0].len())
}

fn init_game_state(nrows: usize, ncols: usize) -> GameState {
    let init_direction = || SnakeDirection::Right;
    let mut game_state = GameState {
        grid: vec![vec![EMPTY as u8; ncols]; nrows],
        timing: Duration::from_secs(1),
        last_direction: init_direction(),
        head: (0, 1),
        tail: (0, 0),
        head_directions: VecDeque::from([]),
    };
    let mut add_snake_cell = |p: CellPos| game_state.grid[p.0][p.1] = SNAKE;
    // Adding the snake in the grid.
    add_snake_cell(game_state.head);
    add_snake_cell(game_state.tail);
    game_state
}

fn print_grid(grid: &Grid) {
    // We sum 2 to consider the vertical lines of each side of the grid.
    let ncols = grid_size(grid).1 + 2;
    let put_cursor_left = || print!("{}", termion::cursor::Left(ncols as u16));
    // Printing the top border.
    for _ in 0..ncols {
        print!("⎯")
    }
    println!();
    // Printing cells.
    for row in grid {
        put_cursor_left();
        print!("|");
        for cell in row {
            print!(
                "{}",
                if *cell == SNAKE {
                    "▮"
                } else if *cell == FOOD {
                    "✸"
                } else {
                    " "
                }
            );
        }
        print!("|\n");
    }
    put_cursor_left();
    // Printing the bottom border.
    for _ in 0..ncols {
        print!("⎺")
    }
}

/// Clears the terminal, prints a message and the game grid.
///
/// TODO: This implementation is far from efficient since it refreshes the entire
/// board at each iteration.
fn refresh_screen(
    stdout: &mut RawTerminal<Stdout>,
    message: &String,
    grid: &Grid,
) {
    write!(
        stdout,
        "{}{}{message}\n{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1),
        termion::cursor::Left(30),
        termion::cursor::Hide,
    )
    .unwrap();
    print_grid(&grid);
    stdout.flush().unwrap();
}

fn calc_position(pos: CellPos, direction: SnakeDirection) -> (i16, i16) {
    let (x, y) = (pos.0 as i16, pos.1 as i16);
    match direction {
        SnakeDirection::Right => (x, y + 1),
        SnakeDirection::Down => (x + 1, y),
        SnakeDirection::Left => (x, y - 1),
        SnakeDirection::Up => (x - 1, y),
    }
}

fn update_snake(
    state: &mut GameState,
    new_direction: SnakeDirection,
) -> IsGameValid {
    let (nrows, ncols) = grid_size(&state.grid);
    let (new_head_x, new_head_y) = calc_position(state.head, new_direction);
    // If the snake's head position goes out of the board boundaries, the game
    // is over.
    if (new_head_x < 0)
        || (new_head_y < 0)
        || (new_head_x >= nrows as i16)
        || (new_head_y >= ncols as i16)
    {
        return false;
    }
    // The game is still valid. We update the head's position and its
    // representation in the grid.
    state.head = (new_head_x as usize, new_head_y as usize);
    state.grid[state.head.0][state.head.1] = SNAKE;
    state.head_directions.push_front(new_direction);
    //We update the tail's position and representation in the grid.
    state.grid[state.tail.0][state.tail.1] = EMPTY;
    state.tail = {
        let (tx, ty) = calc_position(
            state.tail,
            state.head_directions.pop_back().unwrap(),
        );
        (tx as usize, ty as usize)
    };

    true
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout().into_raw_mode().unwrap();
    let mut game = init_game_state(30, 60);
    refresh_screen(&mut stdout, &String::from("Start"), &game.grid);

    for i in 0..10 {
        let new_direction = game.last_direction;
        let is_valid = update_snake(&mut game, new_direction);
        // If the update is valid, we continue playing.
        if is_valid {
            refresh_screen(&mut stdout, &format!("Iteration {i}"), &game.grid);
            thread::sleep(game.timing);
        } else {
            // If the game is in a invalid state, the game is over.
            refresh_screen(&mut stdout, &format!("Game is over"), &game.grid);
            thread::sleep(Duration::from_secs(3));
            break;
        }
    }

    //    let mut stdin_keys = termion::async_stdin().keys();
    //    let mut stdout = io::stdout().into_raw_mode().unwrap();
    //
    //    for i in 0..10 {
    //        write!(stdout, "{0}Iteration {i}\n{0}", termion::cursor::Left(20))
    //            .unwrap();
    //
    //        if let Some(Ok(c)) = stdin_keys.next() {
    //            match c {
    //                Key::Char('q') => break,
    //                Key::Char(c) => println!("{}", c),
    //                Key::Alt(c) => println!("^{}", c),
    //                Key::Ctrl(c) => println!("*{}", c),
    //                Key::Esc => println!("ESC"),
    //                Key::Left => println!("←"),
    //                Key::Right => println!("→"),
    //                Key::Up => println!("↑"),
    //                Key::Down => println!("↓"),
    //                Key::Backspace => println!("×"),
    //                _ => {}
    //            }
    //            stdout.flush().unwrap();
    //        }
    //        thread::sleep(Duration::from_secs(1));
    //    }

    write!(stdout, "{}", termion::cursor::Show).unwrap();
    Ok(())
}
