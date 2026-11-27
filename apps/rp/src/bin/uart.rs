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
use static_cell::StaticCell;

struct Board<UartTx, UartRx> {
    uart_tx: UartTx,
    uart_rx: UartRx,
}

#[emb::executor::main]
async fn main(_spawner: emb::executor::Spawner) {
    let board = init_board();
    let config = edline::ConsoleConfig::default();
    let key_reader = serkey::Reader::new(board.uart_rx);
    let line_buf = {
        static CELL: StaticCell<[u8; 128]> = StaticCell::new();
        CELL.init([0; 128])
    };
    let history_buf = {
        static CELL: StaticCell<[u8; 1024]> = StaticCell::new();
        CELL.init([0; 1024])
    };
    let editor = edline::with_console(edline::tag::CharAttrMonoColor,
        line_buf, history_buf, board.uart_tx, config);
    edline::simple_task(editor, key_reader, false).await;
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
