#![no_std]
#![no_main]

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::io;
mod grid;
use grid::Grid;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

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
