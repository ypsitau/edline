//! UART Echo example using Embassy and async/await.
//!
//! ## Pin Assignment
//! | Raspberry Pi Pico | Function |
//! |------------------|----------|
//! | GPIO0            | UART TX  |
//! | GPIO1            | UART RX  |
#![no_std]
#![no_main]

mod emb {
    pub use embassy_executor as executor;
}
use embedded_io_async as io_async;
use embedded_io_async::{Read as _, Write as _};
use defmt::info;
use static_cell::StaticCell;

struct Board<UartTx, UartRx> {
    uart_tx: UartTx,
    uart_rx: UartRx,
}

#[emb::executor::main]
async fn main(_spawner: emb::executor::Spawner) {
    let board = init_board();
    let fut_edline = task_edline(board.uart_tx, board.uart_rx);
    fut_edline.await;
}

async fn task_edline(mut writer: impl io_async::Write, mut reader: impl io_async::Read) -> ! {
    writer.write_all(b"Starting editor...\r\n").await.ok();
    writer.flush().await.ok();
    let mut line_buf = {
        static STATIC_CELL: StaticCell<[u8; 128]> = StaticCell::new();
        STATIC_CELL.init([0u8; 128])
    };
    let mut history_buf = {
        static STATIC_CELL: StaticCell<[u8; 1024]> = StaticCell::new();
        STATIC_CELL.init([0u8; 1024])
    };
    let mut editor = edline::Editor::with_serial_output(writer, line_buf, history_buf);
    let mut serkey_reader = serkey::Reader::new();
    editor.start().await;
    loop {
        if let Ok(key) = serkey_reader.read_key(&mut reader).await {
            editor.process_key(key).await;
        }
    }
}

//-----------------------------------------------------------------------------
// board-specific implementation
//-----------------------------------------------------------------------------
use embassy_rp as rp;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    UART0_IRQ => rp::uart::BufferedInterruptHandler<rp::peripherals::UART0>;
});

fn init_board() -> Board<impl io_async::Write, impl io_async::Read> {
    let p = rp::init(Default::default());
    let uart = {
        let (pin_tx, pin_rx) = (p.PIN_0, p.PIN_1);
        let tx_buffer = {
            const BUFSIZE: usize = 64;
            static STATIC_CELL: StaticCell<[u8; BUFSIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; BUFSIZE])
        };
        let rx_buffer = {
            const BUFSIZE: usize = 64;
            static STATIC_CELL: StaticCell<[u8; BUFSIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; BUFSIZE])
        };
        let config = rp::uart::Config::default();
        rp::uart::BufferedUart::new(p.UART0, pin_tx, pin_rx, Irqs, tx_buffer, rx_buffer, config)
    };
    let (uart_tx, uart_rx) = uart.split();
    Board { uart_tx, uart_rx }
}
