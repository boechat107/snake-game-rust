use rand::Rng;
use std::{
    collections::VecDeque, io, io::Stdout, io::Write, thread, time::Duration,
};
use termion::{
    event::Key, input::Keys, input::TermRead, raw::IntoRawMode,
    raw::RawTerminal, AsyncReader,
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

enum UserInput {
    Quit,
    Direction(SnakeDirection),
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

const GRID_ROWS: usize = 15;
const GRID_COLUMNS: usize = 30;
const SNAKE: Cell = 1;
const EMPTY: Cell = 0;
const FOOD: Cell = 2;
const QUIT_CHAR: char = 'q';
const MAX_FOOD_AMOUNT: usize = 15;

fn grid_size(grid: &Grid) -> (usize, usize) {
    (grid.len(), grid[0].len())
}

fn add_food(grid: &mut Grid, max_amount: usize) {
    let (nrows, ncols) = grid_size(grid);
    // A random number generator.
    let mut rng = rand::thread_rng();
    // This is a sequence of random food locations like [(x1, y1), (x2, y2)].
    let food_locations = (0..max_amount)
        .map(|_| (rng.gen_range(0..nrows), rng.gen_range(0..ncols)));
    // We add the food to the grid, but only in empty cells.
    for (food_x, food_y) in food_locations {
        if grid[food_x][food_y] == EMPTY {
            grid[food_x][food_y] = FOOD;
        }
    }
}

fn init_game_state(nrows: usize, ncols: usize) -> GameState {
    let init_direction = || SnakeDirection::Right;
    let mut game_state = GameState {
        grid: vec![vec![EMPTY as u8; ncols]; nrows],
        timing: Duration::from_secs(1),
        last_direction: init_direction(),
        head: (0, 1),
        tail: (0, 0),
        head_directions: VecDeque::from([init_direction()]),
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

fn _calc_position(pos: CellPos, direction: SnakeDirection) -> (i16, i16) {
    let (x, y) = (pos.0 as i16, pos.1 as i16);
    match direction {
        SnakeDirection::Right => (x, y + 1),
        SnakeDirection::Down => (x + 1, y),
        SnakeDirection::Left => (x, y - 1),
        SnakeDirection::Up => (x - 1, y),
    }
}

/// Updates the tail position by making its current cell empty and setting the
/// new tail position.
fn _update_snake_tail(state: &mut GameState) {
    state.grid[state.tail.0][state.tail.1] = EMPTY;
    state.tail = {
        let (tx, ty) = _calc_position(
            state.tail,
            state.head_directions.pop_back().unwrap(),
        );
        (tx as usize, ty as usize)
    };
}

fn _update_snake_head(
    state: &mut GameState,
    new_head_x: usize,
    new_head_y: usize,
    new_direction: SnakeDirection,
) {
    state.head = (new_head_x as usize, new_head_y as usize);
    // Before updating the cell content with the snake's head, we store whas
    state.grid[state.head.0][state.head.1] = SNAKE;
    state.head_directions.push_front(new_direction);
}

fn update_snake(
    state: &mut GameState,
    new_direction: SnakeDirection,
) -> IsGameValid {
    let (nrows, ncols) = grid_size(&state.grid);
    let (new_head_x, new_head_y) = _calc_position(state.head, new_direction);
    // If the snake's head position goes out of the board boundaries, the game
    // is over.
    if (new_head_x < 0)
        || (new_head_y < 0)
        || (new_head_x >= nrows as i16)
        || (new_head_y >= ncols as i16)
    {
        return false;
    }
    // We know now that the new position is not out of the game grid. We can
    // safely convert their types.
    let (new_head_x, new_head_y) = (new_head_x as usize, new_head_y as usize);
    let does_head_meets_food = state.grid[new_head_x][new_head_y] == FOOD;
    // We update the head's position and its representation in the grid.
    _update_snake_head(state, new_head_x, new_head_y, new_direction);
    // We update the tail's position only if the head does not meet food. If it
    // does, we want to make the snake grow.
    if !does_head_meets_food {
        _update_snake_tail(state);
    }
    // We keep track of the last direction.
    state.last_direction = new_direction;

    true
}

fn capture_input(stdin_keys: &mut Keys<AsyncReader>) -> Option<UserInput> {
    let mut last_key = None;
    // We consider only the user's last input, except if it is to quit the game.
    for key in stdin_keys {
        last_key = match key {
            Ok(k) => Some(k),
            _ => None,
        };
        if let Some(Key::Char(QUIT_CHAR)) = last_key {
            break;
        }
    }
    match last_key {
        Some(Key::Char(QUIT_CHAR)) => Some(UserInput::Quit),
        Some(Key::Right) => Some(UserInput::Direction(SnakeDirection::Right)),
        Some(Key::Down) => Some(UserInput::Direction(SnakeDirection::Down)),
        Some(Key::Left) => Some(UserInput::Direction(SnakeDirection::Left)),
        Some(Key::Up) => Some(UserInput::Direction(SnakeDirection::Up)),
        _ => None,
    }
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout().into_raw_mode().unwrap();
    let mut stdin_keys = termion::async_stdin().keys();
    let mut game = init_game_state(GRID_ROWS, GRID_COLUMNS);
    add_food(&mut game.grid, MAX_FOOD_AMOUNT);

    refresh_screen(&mut stdout, &String::from("Start"), &game.grid);
    for i in 0.. {
        // We take the user input (if it exists) and check if the user wants to
        // quit the game.
        let user_input = capture_input(&mut stdin_keys);
        if let Some(UserInput::Quit) = user_input {
            break;
        }
        // If the user inputted a new snake direction, we use it; otherwise, we
        // make the snake continue in the same direction.
        let new_direction = match user_input {
            Some(UserInput::Direction(snake_direction)) => snake_direction,
            _ => game.last_direction,
        };
        // We update the snake's position and check if it is valid (the snake
        // is inside the game board).
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

    write!(stdout, "{}", termion::cursor::Show).unwrap();
    Ok(())
}

#[cfg(test)]
mod main_test {
    use super::*;

    #[test]
    fn test_food_gen() {
        let (nrows, ncols, nfood) = (10, 10, 5);
        let mut game = init_game_state(nrows, ncols);
        add_food(&mut game.grid, nfood);
        print_grid(&game.grid);

        let grid_sum: u8 = game.grid.iter().flat_map(|cols| cols.iter()).sum();
        // The food is randomly placed in the grid, and it might be located in
        // the snake place (therefore, ignored). The number of food in the grid
        // may discount the initial snake size (which is 2).
        assert!(grid_sum > (2 * SNAKE + (nfood as u8 - 2) * FOOD))
    }
}
