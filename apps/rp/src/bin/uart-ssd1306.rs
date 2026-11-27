//! Test appllication for edline on Raspberry Pi Pico with UART and SSD1306.
//!
//! ## Pin Assignment
//! | Raspberry Pi Pico | Function |
//! |------------------|----------|
//! | GPIO0            | UART TX  |
//! | GPIO1            | UART RX  |
//! | GPIO6            | TFT LCD RESET |
//! | GPIO7            | TFT LCD DC |
//! | GPIO8            | TFT LCD CS |
//! | GPIO9            | TFT LCD BL |
//! | GPIO10           | TFT LCD CLK |
//! | GPIO11           | TFT LCD MOSI |
//! | GPIO14           | I2C SDA |
//! | GPIO15           | I2C SCL |
#![no_std]
#![no_main]

mod emb {
    pub use embassy_executor as executor;
    pub use embassy_time as time;
}

use rp_uart_display::board_rp::{self, prelude::*};
use embedded_graphics as eg;
use static_cell::StaticCell;

#[emb::executor::main]
async fn main(_spawner: emb::executor::Spawner) {
    let board = board_rp::with_ssd1306::<0>(ssd1306::rotation::DisplayRotation::Rotate0);
    let config = edline::GraphicsConfig::default()
        .font(&eg::mono_font::ascii::FONT_6X12)
        .text_color(From::from(eg::pixelcolor::Rgb888::new(255, 255, 255)))
        .background_color(From::from(eg::pixelcolor::Rgb888::new(0, 0, 128)))
        .ht_cursor(2);
    let key_reader = serkey::Reader::new(board.uart_rx);
    let line_buf = {
        static CELL: StaticCell<[u8; 128]> = StaticCell::new();
        CELL.init([0; 128])
    };
    let history_buf = {
        static CELL: StaticCell<[u8; 1024]> = StaticCell::new();
        CELL.init([0; 1024])
    };
    let rollback_buf = {
        static CELL: StaticCell<[u8; 1024]> = StaticCell::new();
        CELL.init([0; 1024])
    };
    let editor = edline::with_graphics(edline::tag::CharAttrMonoColor,
        line_buf, history_buf, board.display, rollback_buf, config, emb::time::Delay,
        |display| { display.flush().unwrap() });
    edline::simple_task(editor, key_reader, false).await;
}
