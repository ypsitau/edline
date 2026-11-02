pub struct RoundBuffer<'a> {
    buf: &'a mut [u8],
    idx_first: usize,
    idx_last: usize,
    idx_write: usize,
    buf_full: bool,
}

impl<'a> RoundBuffer<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, idx_first: 0, idx_last: 0, idx_write: 0, buf_full: false }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }

    pub fn add(&mut self, src: &[u8]) -> Result<(), ()> {
        if src.len() + 1 > self.capacity() { return Err(()); }
        if self.idx_write >= self.buf.len() { self.idx_write = 0; }
        let idx_write_end = self.idx_write + src.len() + 1;
        if idx_write_end <= self.buf.len() {
            if self.buf_full {
                let mut round_up = false;
                while self.idx_first < idx_write_end && !round_up {
                    while self.buf[self.idx_first] != 0x00 {
                        self.idx_first += 1;
                        if self.idx_first >= self.buf.len() {
                            self.idx_first = 0;
                            round_up = true;
                        }
                    }
                    self.idx_first += 1;
                    if self.idx_first >= self.buf.len() {
                        self.idx_first = 0;
                        round_up = true;
                    }
                }
            }
            self.buf[self.idx_write..idx_write_end - 1].copy_from_slice(src);
            self.buf[idx_write_end - 1] = 0x00;
            self.idx_last = self.idx_write;
            self.idx_write = idx_write_end;
        } else {
            self.buf_full = true;
            let nbytes_copy = self.buf.len() - self.idx_write;
            let nbytes_rest = src.len() - nbytes_copy;
            let idx_write_end = nbytes_rest + 1;
            while self.idx_first < idx_write_end || self.idx_first >= self.idx_write {
                while self.buf[self.idx_first] != 0x00 {
                    self.idx_first += 1;
                    if self.idx_first >= self.buf.len() { self.idx_first = 0; }
                }
                self.idx_first += 1;
                if self.idx_first >= self.buf.len() { self.idx_first = 0; }
            }
            self.buf[self.idx_write..self.idx_write + nbytes_copy].copy_from_slice(&src[..nbytes_copy]);
            self.buf[0..nbytes_rest].copy_from_slice(&src[nbytes_copy..]);
            self.buf[nbytes_rest] = 0x00;
            self.idx_last = self.idx_write;
            self.idx_write = idx_write_end;
        }
        Ok(())
    }

    pub fn print(&self) {
        const NCOLS: usize = 16;
        println!("first:0x{:04x} last:0x{:04x} write:0x{:04x}", self.idx_first, self.idx_last, self.idx_write);
        let mut idx = 0;
        for chunk in self.buf.chunks(NCOLS) {
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
                if idx == self.idx_first { print!("F"); n += 1;}
                if idx == self.idx_last { print!("L"); n += 1; }
                if idx == self.idx_write { print!("W"); n += 1; }
                for _ in n..3 { print!(" "); }
                idx += 1;
            }
            println!();
        }
    }
}
