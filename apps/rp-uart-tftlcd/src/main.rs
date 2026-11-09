//! UART Echo example using Embassy and async/await.
//!
//! ## Pin Assignment
//! | Raspberry Pi Pico | Function |
//! |------------------|----------|
//! | GPIO0            | UART TX  |
//! | GPIO1            | UART RX  |
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
    let fut_edline = edline::simple_task_graphics(board.display, board.uart_rx, false, emb::time::Delay, |_| {});
    fut_edline.await;
}
