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
    let mut stdin = embedded_io_adapters::tokio_1::FromTokio::new(tokio::io::stdin());
    let stdout = embedded_io_adapters::tokio_1::FromTokio::new(tokio::io::stdout());
    let output = edline::SerialOutput::new(stdout);
    let mut line_buf = vec![0u8; 128];
    let mut history_buf = vec![0u8; 1024];
    let mut editor = edline::Editor::new(output, &mut line_buf, &mut history_buf);
    let mut serkey_reader = serkey::Reader::new();
    editor.start().await;
    loop {
        if let Some(key) = serkey_reader.read_key(&mut stdin).await {
            match key {
                serkey::Key::CookedCtrl(0x03) => break,
                _ => editor.process_key(key).await,
            }
        }
    }
    crossterm::terminal::disable_raw_mode()?;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console as wc;
        let handle = wc::GetStdHandle(wc::STD_INPUT_HANDLE);
        wc::SetConsoleMode(handle, mode_saved);
    }
    Ok(())
}
