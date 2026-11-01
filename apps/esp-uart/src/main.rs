//! UART Echo Example
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

#[esp_rtos::main]
async fn main(_spawner: emb::executor::Spawner) {
    let mut board = init_board();
    let fut_edline = task_edline(board.uart_tx, board.uart_rx);
    fut_edline.await;
}

async fn task_edline(mut writer: impl io_async::Write, mut reader: impl io_async::Read) -> ! {
    let mut line_buf = {
        static STATIC_CELL: StaticCell<[u8; 128]> = StaticCell::new();
        STATIC_CELL.init([0u8; 128])
    };
    let mut history_buf = {
        static STATIC_CELL: StaticCell<[u8; 1024]> = StaticCell::new();
        STATIC_CELL.init([0u8; 1024])
    };
    writer.write_all(b"Starting editor...\r\n").await.ok();
    writer.flush().await.ok();
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
use esp_backtrace as _;
use esp_hal as esp;

esp_bootloader_esp_idf::esp_app_desc!();

fn init_board() -> Board<impl io_async::Write, impl io_async::Read> {
    esp_println::logger::init_logger_from_env();
    let p = esp::init(Default::default());
    let (pin_uart_tx, pin_uart_rx) = (p.GPIO1, p.GPIO3);
    let timg0 = esp::timer::timg::TimerGroup::new(p.TIMG0);
    let software_interrupt_control = esp::interrupt::software::SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt_control.software_interrupt0);
    //esp_rtos::start(timg0.timer0, p.FROM_CPU_INTR0);
    let uart = {
        const FIFO_FULL_THRESHOLD: u16 = 64;
        let config = esp::uart::Config::default()
            .with_rx(esp::uart::RxConfig::default()
            .with_fifo_full_threshold(FIFO_FULL_THRESHOLD));
        esp::uart::Uart::new(p.UART0, config).unwrap().with_tx(pin_uart_tx).with_rx(pin_uart_rx).into_async()
    };
    let (uart_rx, uart_tx) = uart.split();
    Board { uart_tx, uart_rx }
}
