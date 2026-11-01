use embedded_io_async as io_async;
use static_cell::StaticCell;

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
    task_edline(stdout, stdin).await;
    crossterm::terminal::disable_raw_mode()?;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console as wc;
        let handle = wc::GetStdHandle(wc::STD_INPUT_HANDLE);
        wc::SetConsoleMode(handle, mode_saved);
    }
    Ok(())
}

async fn task_edline(mut writer: impl io_async::Write, mut reader: impl io_async::Read) {
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
            if let serkey::Key::CookedCtrl(serkey::Ctrl::C) = key {
                break;
            }
            editor.process_key(key).await;
        }
    }
}
