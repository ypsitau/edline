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
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'0'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'1'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'2'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'3'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'4'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[0; 0]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[0; 0]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'5'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'6'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'7'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    round_buf.add(&[b'8'; 17]).unwrap();
    print_buf(&round_buf); print_items(&mut round_buf);
    for i_trial in 0..100 {
        let len = rng.random_range(0..32);
        for i in 0..len {
            buf_write[i] = b'A' + (i_trial % 26) as u8;
        }
        round_buf.add(&buf_write[..len]).unwrap();
        print_buf(&round_buf); print_items(&mut round_buf);
    }
}

pub fn print_buf(round_buf: &edline::RoundBuffer) {
    const NCOLS: usize = 16;
    println!("first:0x{:04x} last:0x{:04x} write:0x{:04x} count:{}",
        round_buf.idx_top(), round_buf.idx_bottom(), round_buf.idx_write(), round_buf.count());
    let mut idx = 0;
    for chunk in round_buf.buf().chunks(NCOLS) {
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        for _ in 0..(NCOLS - chunk.len()) {
            print!("   ");
        }
        for byte in chunk {
            if *byte >= 0x20 && *byte <= 0x7e {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!();
        for _ in 0..chunk.len() {
            let mut n = 0;
            if idx == round_buf.idx_top() { print!("F"); n += 1;}
            if idx == round_buf.idx_bottom() { print!("L"); n += 1; }
            if idx == round_buf.idx_write() { print!("W"); n += 1; }
            for _ in n..3 { print!(" "); }
            idx += 1;
        }
        println!();
    }
}

pub fn print_items(round_buf: &mut edline::RoundBuffer) {
    round_buf.reset_cursor();
    while let Some((first, second, last_item)) = round_buf.move_cursor_forward() {
        println!("(\"{}\", \"{}\")", str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
        if last_item { break; }
    }
    round_buf.reset_cursor();
    while let Some((first, second, last_item)) = round_buf.move_cursor_backward() {
        println!("(\"{}\", \"{}\")", str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
        if last_item { break; }
    }
}
