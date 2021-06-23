#![no_std]
#![no_main]

mod crypto_helpers;
mod utils;

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::io;
use nanos_sdk::screen;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

fn screen_clear() {
    screen::sdk_bagl_hal_draw_rect(0, 0, 0, 128, 64);
    screen::sdk_screen_update();
}

fn screen_clear_and_draw(x_pos: i32, y_pos: i32, bmp: &[u8]) {
    screen_clear();
    screen::sdk_bagl_hal_draw_bitmap_within_rect(x_pos, y_pos, 128, 64, false, bmp);
    screen::sdk_screen_update();
}

fn calculate_new_position(x_pos: &mut i32, y_pos: &mut i32, ev: ButtonEvent) {

    match ev {
        ButtonEvent::RightButtonPress => {
            *x_pos += 1;
            *y_pos += 1;
        },
        ButtonEvent::LeftButtonPress => {
            *x_pos -= 1;
            *y_pos -= 1;
        }
        _ => ()
    };

   if *x_pos > 127 {
       *x_pos = 0;
   } else if *x_pos < 0 {
       *x_pos = 127;
   }

   if *y_pos > 63 {
       *y_pos = 0;
   } else if *y_pos < 0 {
       *y_pos = 63
   }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    let mut x_pos: i32 = 0;
    let mut y_pos = 0;
    let bitmap = [0xFF, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0xA5, 0xA5];

    loop {
        match comm.next_event() {
            io::Event::Button(ButtonEvent::LeftButtonPress) => {
                calculate_new_position(&mut x_pos, &mut y_pos, ButtonEvent::LeftButtonPress);
                screen_clear_and_draw(x_pos, y_pos, &bitmap);
            }
            io::Event::Button(ButtonEvent::RightButtonPress) => {
                calculate_new_position(&mut x_pos, &mut y_pos, ButtonEvent::RightButtonPress);
                screen_clear_and_draw(x_pos, y_pos, &bitmap);
            }
            io::Event::Command::<u8>(_) => (),
            _ => (),
        }
    }
}
