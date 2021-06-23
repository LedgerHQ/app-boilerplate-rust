#![no_std]
#![no_main]

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::exit_app;
use nanos_sdk::io;
use nanos_sdk::screen;
mod grid;
mod logos;
use grid::Grid;
use logos::*;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

fn screen_clear() {
    screen::sdk_bagl_hal_draw_rect(0, 0, 0, 128, 64);
    screen::sdk_screen_update();
}

fn screen_clear_and_draw(x_pos: i32, y_pos: i32, width: u32, height: u32, bmp: &[u8]) {
    screen_clear();
    screen::sdk_bagl_hal_draw_bitmap_within_rect(x_pos, y_pos, width, height, false, bmp);
    screen::sdk_screen_update();
}

#[allow(clippy::manual_range_contains)]
fn get_next_bmp(mut cnt: u8) -> [u8; 1024] {
    cnt %= 50;
    if cnt < 20 {
        LOGO_PASCAL
    } else if cnt >= 20 && cnt < 25 {
        LOGO_RUSTX_0
    } else if cnt >= 25 && cnt < 30 {
        LOGO_RUSTX_1
    } else if cnt >= 30 && cnt < 35 {
        LOGO_RUSTX_0
    } else if cnt >= 35 && cnt < 40 {
        LOGO_RUSTX_1
    } else if cnt >= 40 && cnt < 45 {
        LOGO_RUSTX_0
    } else {
        LOGO_RUSTX_1
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    let mut grid = Grid::new();
    grid.init_draw();

    let x_pos: i32 = 0;
    let y_pos = 0;

    let width: u32 = 128;
    let height: u32 = 64;

    let mut cnt: u8 = 0;

    loop {
        match comm.next_event() {
            io::Event::Button(ButtonEvent::LeftButtonRelease) => {
                grid.select_prev();
            }
            io::Event::Button(ButtonEvent::RightButtonRelease) => {
                grid.select_next();
            }
            io::Event::Button(ButtonEvent::BothButtonsRelease) => {
                if grid.is_finished() {
                    exit_app(0);
                }
                grid.add_mark();

            }
            io::Event::Ticker => {
                if grid.is_finished() {
                    cnt += 1;
                    let bmp = get_next_bmp(cnt);
                    screen_clear_and_draw(x_pos, y_pos, width, height, &bmp);
                }
            }
            io::Event::Command::<u8>(_) => (),
            _ => (),
        }
    }
}
