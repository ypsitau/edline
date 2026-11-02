/*
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
    edline::simple_task(stdout, stdin, true).await;
    crossterm::terminal::disable_raw_mode()?;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console as wc;
        let handle = wc::GetStdHandle(wc::STD_INPUT_HANDLE);
        wc::SetConsoleMode(handle, mode_saved);
    }
    Ok(())
}
*/

fn main() {
    let mut buf = [0u8; 64];
    let mut round_buf = edline::RoundBuffer::new(&mut buf);
    round_buf.print();
    round_buf.add(b"0000000000").unwrap();
    round_buf.print();
    round_buf.add(b"1111111111").unwrap();
    round_buf.print();
    round_buf.add(b"2222222222").unwrap();
    round_buf.print();
    round_buf.add(b"3333333333").unwrap();
    round_buf.print();
    round_buf.add(b"4444444444").unwrap();
    round_buf.print();
    round_buf.add(b"5555555555").unwrap();
    round_buf.print();
    round_buf.add(b"6666666666").unwrap();
    round_buf.print();
    round_buf.add(b"7777777777").unwrap();
    round_buf.print();
    round_buf.add(b"8888888888").unwrap();
    round_buf.print();
    round_buf.add(b"9999999999").unwrap();
    round_buf.print();
    round_buf.add(b"aaaaaaaaaa").unwrap();
    round_buf.print();
    round_buf.add(b"bbbbbbbbbb").unwrap();
    round_buf.print();
    round_buf.add(b"cccccccccc").unwrap();
    round_buf.print();
    round_buf.add(b"dddddddddd").unwrap();
    round_buf.print();
}