use embedded_hal_async as hal_async;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator as eg_sim;

type Display = eg_sim::SimulatorDisplay<eg::pixelcolor::Rgb888>;

#[tokio::main(flavor = "current_thread")]
async fn main() { //-> Result<(), core::convert::Infallible> {
    crossterm::terminal::enable_raw_mode().unwrap();
    #[cfg(windows)]
    let mode_saved = unsafe {
        use windows_sys::Win32::System::Console as wc;
        let handle = wc::GetStdHandle(wc::STD_INPUT_HANDLE);
        let mut mode = 0;
        if wc::GetConsoleMode(handle, &mut mode) != 0 {
            wc::SetConsoleMode(handle, mode | wc::ENABLE_VIRTUAL_TERMINAL_INPUT);
        }
        mode
    };
    print!("Press Ctrl-C to exit\r\n");
    let target = Display::new(Size::new(320, 240));
    let output_settings = eg_sim::OutputSettingsBuilder::new().scale(2).build();
    let mut window = eg_sim::Window::new("edline", &output_settings);
    let stdin = embedded_io_adapters::tokio_1::FromTokio::new(tokio::io::stdin());
    let (update_sender, mut update_receiver) = tokio::sync::watch::channel(target.clone());
    let task_window = async {
        if update_receiver.changed().await.is_err() { return; }
        window.update(&update_receiver.borrow_and_update());
        let mut event_interval = tokio::time::interval(std::time::Duration::from_millis(16));
        loop {
            tokio::select! {
                result = update_receiver.changed() => {
                    if result.is_err() { break; }
                    window.update(&update_receiver.borrow_and_update());
                }
                _ = event_interval.tick() => {
                    if window.events().any(|e| e == eg_sim::SimulatorEvent::Quit) { break; }
                }
            }
        }
    };
    let key_reader = serkey::Reader::new(stdin);
    let task_edline = edline::simple_task_graphics(target, key_reader, true, ThreadDelay,
        |target| request_update(&update_sender, target));
    tokio::select! {
        _ = task_window => {}
        _ = task_edline => {}
    }
    crossterm::terminal::disable_raw_mode().unwrap();
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console as wc;
        let handle = wc::GetStdHandle(wc::STD_INPUT_HANDLE);
        wc::SetConsoleMode(handle, mode_saved);
    }
}

fn request_update(sender: &tokio::sync::watch::Sender<Display>, target: &Display) {
    sender.send_replace(target.clone());
}

pub struct ThreadDelay;

impl hal_async::delay::DelayNs for ThreadDelay {
    async fn delay_ns(&mut self, ns: u32) {
        tokio::time::sleep(std::time::Duration::from_nanos(u64::from(ns))).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eg::pixelcolor::Rgb888;

    #[tokio::test]
    async fn update_requests_deliver_latest_display_snapshot() {
        let mut target = Display::new(Size::new(2, 1));
        let (sender, mut receiver) = tokio::sync::watch::channel(target.clone());

        Pixel(Point::new(0, 0), Rgb888::RED).draw(&mut target).unwrap();
        request_update(&sender, &target);
        Pixel(Point::new(1, 0), Rgb888::GREEN).draw(&mut target).unwrap();
        request_update(&sender, &target);
        Pixel(Point::new(0, 0), Rgb888::BLUE).draw(&mut target).unwrap();

        receiver.changed().await.unwrap();
        let display = receiver.borrow_and_update();
        assert_eq!(display.get_pixel(Point::new(0, 0)), Rgb888::RED);
        assert_eq!(display.get_pixel(Point::new(1, 0)), Rgb888::GREEN);
    }
}
