use rand::RngExt;

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
    let mut rng = rand::rng();
    let mut buf_write = [0u8; 64];
    let mut buf = [0u8; 64];
    let mut round_buf = edline::RoundBuffer::new(&mut buf);
    round_buf.print();
    round_buf.add(&[b'0'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[b'1'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[b'2'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[b'3'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[b'4'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[0; 0]).unwrap();
    round_buf.print();
    round_buf.add(&[0; 0]).unwrap();
    round_buf.print();
    round_buf.add(&[b'5'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[b'6'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[b'7'; 17]).unwrap();
    round_buf.print();
    round_buf.add(&[b'8'; 17]).unwrap();
    round_buf.print();
    for i_trial in 0..100 {
        let len = rng.random_range(0..32);
        for i in 0..len {
            buf_write[i] = b'A' + (i_trial % 26) as u8;
        }
        round_buf.add(&buf_write[..len]).unwrap();
        round_buf.print();
    }
    for (part1, part2) in round_buf.iter() {
        println!("part1: {:x?}, part2: {:x?}", part1, part2);
    }
}
