#![no_std]
#![no_main]

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::io;
use nanos_sdk::screen;

mod snake;

use snake::Game;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

fn screen_clear() {
    screen::sdk_bagl_hal_draw_rect(0, 0, 0, 128, 64);
    screen::sdk_screen_update();
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    let mut game = Game::new();

    loop {
        match comm.next_event() {
            io::Event::Button(ButtonEvent::LeftButtonRelease) => game.turn_left(),
            io::Event::Button(ButtonEvent::RightButtonRelease) => game.turn_right(),
            io::Event::Button(ButtonEvent::BothButtonsRelease) => {}
            io::Event::Ticker => game.step(),
            io::Event::Command::<u8>(_) => (),
            _ => (),
        }
    }
}
