#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
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
    let key_reader = serkey::Reader::new(stdin);
    edline::simple_task_stream(stdout, key_reader, true).await;
    crossterm::terminal::disable_raw_mode()?;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console as wc;
        let handle = wc::GetStdHandle(wc::STD_INPUT_HANDLE);
        wc::SetConsoleMode(handle, mode_saved);
    }
    Ok(())
}
