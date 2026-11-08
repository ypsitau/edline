use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator as eg_sim;

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
    let stdout = embedded_io_adapters::tokio_1::FromTokio::new(tokio::io::stdout());
    let stdin = embedded_io_adapters::tokio_1::FromTokio::new(tokio::io::stdin());
    edline::simple_task_stream(stdout, stdin, true).await;
    crossterm::terminal::disable_raw_mode().unwrap();
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console as wc;
        let handle = wc::GetStdHandle(wc::STD_INPUT_HANDLE);
        wc::SetConsoleMode(handle, mode_saved);
    }
    //Ok(())
    /*
    let mut target = eg_sim::SimulatorDisplay::<eg::pixelcolor::Rgb888>::new(Size::new(320, 240));
    let output_settings = eg_sim::OutputSettingsBuilder::new().scale(2).build();
    let mut window = eg_sim::Window::new("text_fonts", &output_settings);
    window.show_static(&target);
    Ok(())
    */
}
