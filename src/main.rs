#![no_std]
#![no_main]

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::exit_app;
use nanos_sdk::io;
use nanos_sdk::screen;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

const GRID_SIZE: u8 = 9;

// Could be enums ?
const BLACK: u32 = 0;
const WHITE: u32 = 1;

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Cross,
    Nought,
}

impl Cell {
    fn draw(&self, cell_num: u8, selected: bool) {
        let mut x = (32 + (cell_num % 3) * 21) as i32;
        let mut y = ((cell_num / 3) * 21) as i32;
        let color;

        if selected {
            screen::sdk_bagl_hal_draw_rect(WHITE, x, y, 19, 19);
            color = BLACK;
        } else {
            screen::sdk_bagl_hal_draw_rect(BLACK, x, y, 19, 19);
            color = WHITE;
        }

        match &self {
            Self::Empty => {}
            Self::Cross => {
                x += 6;
                y += 19 / 2;
                screen::sdk_bagl_hal_draw_rect(color, x, y, 8, 1);
            }
            Self::Nought => {
                y += 6;
                x += 19 / 2;
                screen::sdk_bagl_hal_draw_rect(color, x, y, 1, 8);
            }
        }
    }
}

struct Grid {
    cells: [Cell; 9],
    turn: u8,
    selected: u8,
    finished: bool,
    // could use refs to bitmaps for noughts and crosses
}

impl Grid {
    fn new() -> Self {
        Self {
            cells: [Cell::Empty; GRID_SIZE as usize],
            turn: 0,
            selected: 0,
            finished: false,
        }
    }

    fn init_draw(&self) {
        screen::sdk_bagl_hal_draw_rect(BLACK, 0, 0, 128, 64);

        // TOOD: draw borders ?

        // draw columns
        screen::sdk_bagl_hal_draw_rect(WHITE, 20 + 32, 0, 1, 64);
        screen::sdk_bagl_hal_draw_rect(WHITE, 40 + 32, 0, 1, 64);

        // draw lines
        screen::sdk_bagl_hal_draw_rect(WHITE, 32, 20, 64, 1);
        screen::sdk_bagl_hal_draw_rect(WHITE, 32, 40, 64, 1);

        self.cells[0].draw(0, true);
        screen::sdk_screen_update();
    }

    /// Draw the mark (`Cross` or `Nought`). Does nothing if select cell is `Empty`.
    fn add_mark(&mut self) {
        if self.finished {
            exit_app(0);
        }

        let cell = &mut self.cells[self.selected as usize];
        if *cell == Cell::Empty {
            if self.turn % 2 == 0 {
                *cell = Cell::Cross;
            } else {
                *cell = Cell::Nought;
            }
            self.turn += 1;
        }
        cell.draw(self.selected, true);
        screen::sdk_screen_update();

        if self.player_has_won() || self.is_full() {
            self.finished = true
        }
    }

    fn select_next(&mut self) {
        if self.finished {
            return 
        }
        // Remove the highlight of currently selected cell.
        let cell = self.cells[self.selected as usize];
        cell.draw(self.selected, false);

        if self.selected == GRID_SIZE - 1 {
            self.selected = 0;
        } else {
            self.selected += 1;
        }

        // Draw it (highlighted)
        let cell = self.cells[self.selected as usize];
        cell.draw(self.selected, true);
        screen::sdk_screen_update();
    }

    fn select_prev(&mut self) {
        if self.finished {
            return 
        }
        // Remove the highlight of currently selected cell.
        let cell = self.cells[self.selected as usize];
        cell.draw(self.selected, false);

        // Select next cell
        if self.selected == 0 {
            self.selected = GRID_SIZE - 1;
        } else {
            self.selected -= 1;
        }
        // Draw it (highlighted)
        let cell = self.cells[self.selected as usize];
        cell.draw(self.selected, true);
        screen::sdk_screen_update();
    }

    // Returns true if a row is complete.
    fn check_rows(&self) -> bool {
        for i in 0..3 {
            let check = self.cells[i * 3];
            if check == Cell::Empty {
                continue;
            }
            if self.cells[i * 3 + 1..i * 3 + 3]
                .iter()
                .all(|&cell| cell == check)
            {
                return true;
            }
        }
        false
    }

    // Returns true if a column is complete.
    fn check_columns(&self) -> bool {
        for i in 0..3 {
            let check = self.cells[i];
            if check == Cell::Empty {
                continue;
            }
            let double = [self.cells[i + 3], self.cells[i + 6]];
            if double.iter().all(|&cell| cell == check) {
                return true;
            }
        }
        false
    }

    // Returns true if a diagonal is complete.
    fn check_diagonals(&self) -> bool {
        let check = self.cells[0];
        if check != Cell::Empty {
            let double = [self.cells[4], self.cells[8]];
            if double.iter().all(|&cell| cell == check) {
                return true;
            }
            let check = self.cells[2];
            if check != Cell::Empty {
                let double = [self.cells[4], self.cells[6]];
                if double.iter().all(|&cell| cell == check) {
                    return true;
                }
            }
        }
        false
    }

    fn player_has_won(&self) -> bool {
        self.check_rows() || self.check_columns() || self.check_diagonals()
    }

    fn is_full(&self) -> bool {
        self.turn == GRID_SIZE
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    let mut grid = Grid::new();
    grid.init_draw();

    loop {
        match comm.next_event() {
            io::Event::Button(ButtonEvent::LeftButtonRelease) => {
                grid.select_prev();
            }
            io::Event::Button(ButtonEvent::RightButtonRelease) => {
                grid.select_next();
            }
            io::Event::Button(ButtonEvent::BothButtonsRelease) => {
                grid.add_mark();
            }
            io::Event::Command::<u8>(_) => (),
            _ => (),
        }
    }
}
