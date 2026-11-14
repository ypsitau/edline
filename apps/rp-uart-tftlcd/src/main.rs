//! Test appllication for edline on Raspberry Pi Pico with UART and TFT LCD.
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

mod board_rp;

mod emb {
    pub use embassy_executor as executor;
    pub use embassy_time as time;
}

#[emb::executor::main]
async fn main(_spawner: emb::executor::Spawner) {
    use mipidsi::options as opts;
    let board = board_rp::init_with_mipidsi::<1, _>(mipidsi::models::ST7789, (240, 320),
        opts::Orientation::new().rotate(opts::Rotation::Deg180),
        opts::ColorOrder::Rgb, opts::ColorInversion::Inverted);
    let key_reader = serkey::Reader::new(board.uart_rx);
    let fut_edline = edline::simple_task_graphics(board.display, key_reader, false, emb::time::Delay, |_| {});
    fut_edline.await;
}
