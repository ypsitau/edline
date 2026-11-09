pub mod prelude {
    pub use super::Flushable as _;
}
mod emb {
    pub use embassy_embedded_hal as hal;
    pub use embassy_sync as sync;
    pub use embassy_time as time;
}
use embedded_io_async as io_async;
use embedded_hal_1 as hal;
use embedded_hal_async as hal_async;
use core::cell::RefCell;
use static_cell::StaticCell;
use embedded_graphics as eg;

use embassy_rp as rp;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    UART0_IRQ => rp::uart::BufferedInterruptHandler<rp::peripherals::UART0>;
});

pub type BlockingMutexNoop<T> =
    emb::sync::blocking_mutex::Mutex<emb::sync::blocking_mutex::raw::NoopRawMutex, T>;

#[allow(dead_code)]
pub struct BoardWithMipidsi<const N_I2C_DEVS: usize, Display, I2c, OutputPin, InputPin, UartTx, UartRx> {
    pub display: Display,
    pub pin_display_bl: OutputPin,
    pub pin_sw: InputPin,
    pub i2c_devs: [I2c; N_I2C_DEVS],
    pub uart_tx: UartTx,
    pub uart_rx: UartRx,
}

#[allow(dead_code)]
pub fn init_with_mipidsi<'d, const N_I2C_DEVS: usize, DisplayModel>(display_model: DisplayModel, size: (u16, u16), orientation: mipidsi::options::Orientation,
    color_order: mipidsi::options::ColorOrder, color_inversion: mipidsi::options::ColorInversion)
    -> BoardWithMipidsi<
        N_I2C_DEVS,
        impl eg::draw_target::DrawTarget<Color = eg::pixelcolor::Rgb565, Error: core::fmt::Debug>,
        impl hal::i2c::I2c,
        impl hal::digital::OutputPin,
        impl hal::digital::InputPin + hal_async::digital::Wait,
        impl io_async::Write,
        impl io_async::Read,
    >
where
    DisplayModel: mipidsi::models::Model<ColorFormat = eg::pixelcolor::Rgb565>,
{
    let p = rp::init(Default::default());
    let (pin_uart0_tx, pin_uart0_rx) = (p.PIN_0, p.PIN_1);
    let (pin_display_reset, pin_display_dc, pin_display_cs, pin_display_bl)
        = (p.PIN_6, p.PIN_7, p.PIN_8, p.PIN_9);
    let (pin_spi1_clk, pin_spi1_mosi) = (p.PIN_10, p.PIN_11);
    let pin_sw = p.PIN_16;
    let (pin_i2c1_sda, pin_i2c1_scl) = (p.PIN_14, p.PIN_15);
    let mutex_i2c1 = {
        type I2c1 = rp::i2c::I2c<'static, rp::peripherals::I2C1, rp::i2c::Blocking>;
        let mut config = rp::i2c::Config::default();
        config.frequency = 400_000;
        static STATIC_CELL: StaticCell<BlockingMutexNoop<RefCell<I2c1>>> = StaticCell::new();
        STATIC_CELL.init(BlockingMutexNoop::new(RefCell::new(
            I2c1::new_blocking(p.I2C1, pin_i2c1_scl, pin_i2c1_sda, config))))
    };
    let mutex_spi1 = {
        type Spi = rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Blocking>;
        static STATIC_CELL: StaticCell<BlockingMutexNoop<RefCell<Spi>>> = StaticCell::new();
        STATIC_CELL.init(BlockingMutexNoop::new(RefCell::new(
            Spi::new_blocking_txonly(p.SPI1, pin_spi1_clk, pin_spi1_mosi, Default::default()))))
    };
    let pin_display_reset = rp::gpio::Output::new(pin_display_reset, rp::gpio::Level::Low);
    let pin_display_dc = rp::gpio::Output::new(pin_display_dc, rp::gpio::Level::Low);
    let pin_display_cs = rp::gpio::Output::new(pin_display_cs, rp::gpio::Level::High);
    let pin_display_bl = rp::gpio::Output::new(pin_display_bl, rp::gpio::Level::High);
    let pin_sw = rp::gpio::Input::new(pin_sw, rp::gpio::Pull::Up);
    let mipidsi_spi_interface = {
        let spi_dev = {
            let mut config = rp::spi::Config::default();
            config.frequency = 64_000_000;
            config.phase = rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = rp::spi::Polarity::IdleHigh;
            emb::hal::shared_bus::blocking::spi::SpiDeviceWithConfig::new(mutex_spi1, pin_display_cs, config)
        };
        let spi_buf = {
            const SPI_BUF_SIZE: usize = 320;
            static STATIC_CELL: StaticCell<[u8; SPI_BUF_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; SPI_BUF_SIZE])
        };
        mipidsi::interface::SpiInterface::new(spi_dev, pin_display_dc, spi_buf)
    };
    let display = mipidsi::Builder::new(display_model, mipidsi_spi_interface)
        .display_size(size.0, size.1).reset_pin(pin_display_reset)
        .orientation(orientation).color_order(color_order).invert_colors(color_inversion)
        .init(&mut emb::time::Delay).unwrap();
    let i2c_devs = core::array::from_fn(|_| emb::hal::shared_bus::blocking::i2c::I2cDevice::new(mutex_i2c1));
    let uart = {
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
        rp::uart::BufferedUart::new(p.UART0, pin_uart0_tx, pin_uart0_rx, Irqs, tx_buffer, rx_buffer, config)
    };
    let (uart_tx, uart_rx) = uart.split();
    BoardWithMipidsi { display, pin_display_bl, pin_sw, i2c_devs, uart_tx, uart_rx }
}

#[allow(dead_code)]
pub trait Flushable {
    type Error: core::fmt::Debug;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

impl<DI, SIZE> Flushable for ssd1306::Ssd1306<DI, SIZE, ssd1306::mode::BufferedGraphicsMode<SIZE>>
where
    DI: ssd1306::prelude::WriteOnlyDataCommand,
    SIZE: ssd1306::size::DisplaySize,
{
    type Error = <Self as ssd1306::prelude::DisplayConfig>::Error;
    fn flush(&mut self) -> Result<(), Self::Error> {
        ssd1306::Ssd1306::flush(self)
    }
}

#[allow(dead_code)]
pub struct BoardWithSSD1306<const N_I2C_DEVS: usize, Display, I2c, InputPin, UartTx, UartRx> {
    pub display: Display,
    pub pin_sw: InputPin,
    pub i2c_devs: [I2c; N_I2C_DEVS],
    pub uart_tx: UartTx,
    pub uart_rx: UartRx,
}

#[allow(dead_code)]
pub fn init_with_ssd1306<'d, const N_I2C_DEVS: usize>(rotation: ssd1306::rotation::DisplayRotation)
    -> BoardWithSSD1306<
        N_I2C_DEVS,
        impl eg::draw_target::DrawTarget<Color = eg::pixelcolor::BinaryColor, Error: core::fmt::Debug> + Flushable,
        impl hal::i2c::I2c,
        impl hal::digital::InputPin + hal_async::digital::Wait,
        impl io_async::Write,
        impl io_async::Read,
    >
{
    use ssd1306::prelude::*;
    let p = rp::init(Default::default());
    let (pin_uart0_tx, pin_uart0_rx) = (p.PIN_0, p.PIN_1);
    let pin_sw = p.PIN_16;
    let (pin_i2c1_sda, pin_i2c1_scl) = (p.PIN_14, p.PIN_15);
    let mutex_i2c1 = {
        let mut config = rp::i2c::Config::default();
        config.frequency = 400_000;
        type I2c1 = rp::i2c::I2c<'static, rp::peripherals::I2C1, rp::i2c::Blocking>;
        static STATIC_CELL: StaticCell<BlockingMutexNoop<RefCell<I2c1>>> = StaticCell::new();
        STATIC_CELL.init(BlockingMutexNoop::new(RefCell::new(
            I2c1::new_blocking(p.I2C1, pin_i2c1_scl, pin_i2c1_sda, config))))
    };
    let pin_sw = rp::gpio::Input::new(pin_sw, rp::gpio::Pull::Up);
    let i2c_devs = core::array::from_fn(|_| emb::hal::shared_bus::blocking::i2c::I2cDevice::new(mutex_i2c1));
    let mut display = {
        let i2c_dev = emb::hal::shared_bus::blocking::i2c::I2cDevice::new(mutex_i2c1);
        let interface = ssd1306::I2CDisplayInterface::new(i2c_dev);
        let size = ssd1306::size::DisplaySize128x64;
        ssd1306::Ssd1306::new(interface, size, rotation).into_buffered_graphics_mode()
    };
    let uart = {
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
        rp::uart::BufferedUart::new(p.UART0, pin_uart0_tx, pin_uart0_rx, Irqs, tx_buffer, rx_buffer, config)
    };
    let (uart_tx, uart_rx) = uart.split();
    display.init().unwrap();
    BoardWithSSD1306 { display, pin_sw, i2c_devs, uart_tx, uart_rx }
}
