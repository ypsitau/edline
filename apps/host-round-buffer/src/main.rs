use rand::RngExt;

fn main() {
    let mut rng = rand::rng();
    let mut buf_write = [0u8; 64];
    let mut buf = [0u8; 64];
    let mut round_buf = edline::RoundBuffer::new(&mut buf);
    print_report(&mut round_buf);
    round_buf.push(&[b'0'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'1'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'2'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'3'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'4'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[0; 0]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[0; 0]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'5'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'6'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'7'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'8'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'9'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[0; 0]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[0; 0]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'a'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'b'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'c'; 17]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'd'; 7]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[0; 0]).unwrap();
    print_report(&mut round_buf);
    round_buf.push(&[b'#'; 63]).unwrap();
    print_report(&mut round_buf);
    for i_trial in 0..1000 {
        let len = rng.random_range(0..63);
        for i in 0..len {
            buf_write[i] = b'A' + (i_trial % 26) as u8;
        }
        round_buf.push(&buf_write[..len]).unwrap();
        print_report(&mut round_buf);
    }
}

pub fn print_report(round_buf: &mut edline::RoundBuffer) {
    println!("-------------------------------------------------------------------");
    print_buf(round_buf);
    print_items(round_buf);
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
            if idx == round_buf.idx_top() { print!("T"); n += 1;}
            if idx == round_buf.idx_bottom() { print!("B"); n += 1; }
            if idx == round_buf.idx_write() { print!("W"); n += 1; }
            for _ in n..3 { print!(" "); }
            idx += 1;
        }
        println!();
    }
}

pub fn print_items(round_buf: &edline::RoundBuffer) {
    let mut items_fwd: Vec<String> = Vec::new();
    println!("Items (forward):");
    let mut nitems = 0;
    let mut idx_item = edline::RoundBuffer::IDX_NONE;
    while round_buf.seek_item_forward(&mut idx_item) {
        let (first, second) = round_buf.get_item(idx_item).unwrap();
        let (first, second) = (str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
        let item = first.to_string() + second;
        println!("\"{}\" (\"{}\" + \"{}\")", item, first, second);
        items_fwd.push(item);
        nitems += 1;
    }
    assert_eq!(nitems, round_buf.count(), "item count mismatch");
    let mut items_bwd: Vec<String> = Vec::new();
    println!();
    println!("Items (backward):");
    let mut idx_item = edline::RoundBuffer::IDX_NONE;
    while round_buf.seek_item_backward(&mut idx_item) {
        let (first, second) = round_buf.get_item(idx_item).unwrap();
        let (first, second) = (str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
        let item = first.to_string() + second;
        println!("\"{}\" (\"{}\" + \"{}\")", item, first, second);
        items_bwd.push(item);
    }
    // items_fwd is compared from the start, items_bwd from the end
    assert_eq!(items_fwd.len(), items_bwd.len(), "item count mismatch");
    for (i, (fwd, bwd)) in items_fwd.iter().zip(items_bwd.iter().rev()).enumerate() {
        assert_eq!(fwd, bwd, "item mismatch at index {}", i);
    }
}
