//! USB Device CDC example using Embassy and async/await.
#![no_std]
#![no_main]

mod emb {
    pub use embassy_executor as executor;
    pub use embassy_futures as futures;
    pub use embassy_usb as usb;
}
use embedded_io_async as io_async;
use core::sync::atomic;
use defmt::info;
use static_cell::StaticCell;

struct Board<UsbDriver> {
    usb_driver: UsbDriver,
}

#[emb::executor::main]
async fn main(_spawner: emb::executor::Spawner) {
    let board = init_board();
    let mut usb_builder = {
        const VID: u16 = 0xc0de;
        const PID: u16 = 0xcafe;
        const CONFIG_DESCRIPTOR_SIZE: usize = 256;
        const BOS_DESCRIPTOR_SIZE: usize = 256;
        const MSOS_DESCRIPTOR_SIZE: usize = 256;
        const CONTROL_BUF_SIZE: usize = 64;
        let mut usb_config = emb::usb::Config::new(VID, PID);
        usb_config.manufacturer = Some("Embassy");
        usb_config.product = Some("hoge");
        usb_config.serial_number = Some("12345678");
        usb_config.max_power = 100;
        usb_config.max_packet_size_0 = CONTROL_BUF_SIZE as u8;
        let config_descriptor_buf = {
            static STATIC_CELL: StaticCell<[u8; CONFIG_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; CONFIG_DESCRIPTOR_SIZE])
        };
        let bos_descriptor_buf = {
            static STATIC_CELL: StaticCell<[u8; BOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; BOS_DESCRIPTOR_SIZE])
        };
        let msos_descriptor_buf = {
            static STATIC_CELL: StaticCell<[u8; MSOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; MSOS_DESCRIPTOR_SIZE])
        };
        let control_buf = {
            static STATIC_CELL: StaticCell<[u8; CONTROL_BUF_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; CONTROL_BUF_SIZE])
        };
        let usb_handler = {
            static STATIC_CELL: StaticCell<USBHandler> = StaticCell::new();
            STATIC_CELL.init(USBHandler::new())
        };
        let mut usb_builder = emb::usb::Builder::new(board.usb_driver, usb_config,
            config_descriptor_buf, bos_descriptor_buf, msos_descriptor_buf, control_buf);
        usb_builder.handler(usb_handler);
        usb_builder
    };
    let (cdc_sender, cdc_receiver) = {
        const MAX_PACKET_SIZE: usize = 64;
        let state = {
            static STATE: StaticCell<emb::usb::class::cdc_acm::State> = StaticCell::new();
            STATE.init(emb::usb::class::cdc_acm::State::new())
        };
        let cdc_driver = emb::usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, MAX_PACKET_SIZE as u16);
        let (cdc_sender, cdc_receiver) = cdc_driver.split();
        let cdc_receiver = {
            let buf = {
                static STATIC_CELL: StaticCell<[u8; MAX_PACKET_SIZE]> = StaticCell::new();
                STATIC_CELL.init([0u8; MAX_PACKET_SIZE])
            };
            cdc_receiver.into_buffered(buf)
        };
        (cdc_sender, cdc_receiver)
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_echo = task_edline(cdc_sender, cdc_receiver);
    emb::futures::join::join(fut_usb, fut_echo).await;
}

async fn task_edline(mut writer: impl io_async::Write, mut reader: impl io_async::Read) -> ! {
    info!("Starting echo task");
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

struct USBHandler {
    configured: atomic::AtomicBool,
}

impl USBHandler {
    fn new() -> Self {
        USBHandler { configured: atomic::AtomicBool::new(false), }
    }
}

impl emb::usb::Handler for USBHandler {
    /// Called when the USB device has been enabled or disabled.
    fn enabled(&mut self, enabled: bool) {
        info!("usb::Handler.enabled({})", enabled);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called after a USB reset after the bus reset sequence is complete.
    fn reset(&mut self) {
        info!("usb::Handler.reset()");
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has set the address of the device to `addr`.
    fn addressed(&mut self, addr: u8) {
        info!("usb::Handler.addressed(addr: {})", addr);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has enabled or disabled the configuration of the device.
    fn configured(&mut self, configured: bool) {
        info!("usb::Handler.configured(configured: {})", configured);
        self.configured.store(configured, atomic::Ordering::Relaxed);
    }
}

//-----------------------------------------------------------------------------
// board-specific implementation
//-----------------------------------------------------------------------------
use {defmt_rtt as _, panic_probe as _};
use embassy_rp as rp;

rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => rp::usb::InterruptHandler<rp::peripherals::USB>;
});

fn init_board() -> Board<impl emb::usb::driver::Driver<'static>> {
    let p = rp::init(Default::default());
    let usb_driver = rp::usb::Driver::new(p.USB, Irqs);
    Board { usb_driver }
}
