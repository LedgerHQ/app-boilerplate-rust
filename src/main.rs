#![no_std]
#![no_main]

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::io;
use nanos_sdk::screen;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

// fn screen_clear() {
//     screen::sdk_bagl_hal_draw_rect(0, 0, 0, 128, 64);
//     screen::sdk_screen_update();
// }

fn screen_clear_and_draw(x_pos: i32, y_pos: i32, bmp: &[u8]) {
    screen::sdk_bagl_hal_draw_bitmap_within_rect(x_pos, y_pos, 128, 64, false, bmp);
    screen::sdk_screen_update();
}

use core::f32::consts::PI;
use libm::{cosf, sinf, sqrtf};

#[allow(dead_code)]
fn seph_setup_ticker(interval_ms: u16) {
    let ms = interval_ms.to_be_bytes();
    nanos_sdk::seph::seph_send(&[0x4e, 0, 2, ms[0], ms[1]]);
}

// t is the frame index. increment it for each tick.
// x is a value from 0 to 128
// y is a value from 0 to 64
fn get_pixel_color(t: u32, x: u32, y: u32) -> bool {
    jumping_blob(t as f32 / 30.0, (x as f32 / 64. - 0.5, y as f32 / 64.))
}

fn jumping_blob(t: f32, o: (f32, f32)) -> bool {
    let mut p = o;
    let radius = 0.18;
    let smoothing = 0.15;
    let dist = 0.26;
    p.0 -= 0.5;
    p.1 -= 0.5;
    p.1 *= -1.0;
    p = p_r(p, PI / 2.0);
    let q = p;
    p = p_r(p, -t);
    let s = f_op_difference_round(
        f_op_union_round(
            q.0,
            length((p.0 + dist, p.1)) - radius,
            smoothing,
        ),
        length((p.0 - dist, p.1)) - radius,
        smoothing,
    );
    return s >= 0.0;
}
fn p_r(p: (f32, f32), a: f32) -> (f32, f32) {
    (
        cosf(a) * p.0 + sinf(a) * p.1,
        cosf(a) * p.1 - sinf(a) * p.0,
    )
}
fn length(l: (f32, f32)) -> f32 {
    sqrtf(l.0 * l.0 + l.1 * l.1)
}
fn f_op_union_round(a: f32, b: f32, r: f32) -> f32 {
    r.max(a.min(b))
        - length(((r - a).max(0.), (r - b).max(0.)))
}
fn f_op_intersection_round(a: f32, b: f32, r: f32) -> f32 {
    (-r).min(a.max(b))
        + length(((r + a).max(0.), (r + b).max(0.)))
}
fn f_op_difference_round(a: f32, b: f32, r: f32) -> f32 {
    f_op_intersection_round(a, -b, r)
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();
    let mut b = [0u8; 128*8];
    let mut frame_n = 0;
    let mut c = 0;


    #[cfg(not(feature = "speculos"))]
    // seph_setup_ticker(16); // 60 fps
    // seph_setup_ticker(32); // 30 fps
    // seph_setup_ticker(64); // 15 fps
    seph_setup_ticker(64); // 15 fps

    for i in 0..512 {
        b[i] = 0xff;
    }
    screen_clear_and_draw(0, 0, &b);

    // bitmap[event_count] = 0;
    // event_count += 1;
    loop {
        // screen::sdk_screen_update();

        // Screen is:

        //    b[0]  |  b[1]  |  b[2]  | ...
        //   b[128] | b[129] |
        //   ...

        // (x, y) = (128*y + x/8) & (1<<(x&7))

        match comm.next_event::<u8>() {
            io::Event::Button(ButtonEvent::RightButtonRelease) => nanos_sdk::exit_app(0),
            io::Event::Ticker => {
                c += 1;
                // if c % 10 == 0 
                {
                    screen_clear_and_draw(0, 0, &b);
                    frame_n += 1;
                    for y in 0..64 {
                        for x in 0..128/8 {
                            let i = (16 * y + x) as usize; 
                            let mut t = 0u8;
                            for z in 0..8 {
                                let p = get_pixel_color(frame_n, 8*x + z, y) as u8;
                                t |= p << z;
                            }
                            b[i] = t;
                        }
                    }
                }
            },
            _ => (),
        }
    }
}
