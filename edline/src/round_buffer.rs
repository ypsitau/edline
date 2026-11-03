pub struct RoundBuffer<'a> {
    buf: &'a mut [u8],
    idx_first: usize,
    idx_last: usize,
    idx_write: usize,
    buf_full: bool,
    nitems: usize,
}

impl<'a> RoundBuffer<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, idx_first: 0, idx_last: 0, idx_write: 0, buf_full: false, nitems: 0 }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }
    pub fn is_empty(&self) -> bool { self.nitems == 0 }
    pub fn count(&self) -> usize { self.nitems }
    pub fn add(&mut self, src: &[u8]) -> Result<(), ()> {
        println!("Adding src of length: {}", src.len());
        if src.len() + 1 > self.capacity() { return Err(()); }
        let idx_write_end = self.idx_write + src.len() + 1;
        if idx_write_end <= self.buf.len() {
            if self.buf_full {
                while self.idx_write <= self.idx_first && self.idx_first < idx_write_end && self.nitems > 0 {
                    let mut idx = self.idx_first;
                    while self.buf[idx] != 0x00 {
                        idx += 1;
                        if idx >= self.buf.len() { idx = 0; }
                    }
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                    self.idx_first = idx;
                    println!("nitems: {}", self.nitems);
                    self.nitems -= 1;
                }
            }
            self.buf[self.idx_write..idx_write_end - 1].copy_from_slice(src);
            self.buf[idx_write_end - 1] = 0x00;
            self.idx_last = self.idx_write;
            if self.nitems == 0 { self.idx_first = self.idx_write; }
            self.idx_write = idx_write_end;
            self.nitems += 1;
        } else {
            self.buf_full = true;
            let nbytes_copy = self.buf.len() - self.idx_write;
            let nbytes_rest = src.len() - nbytes_copy;
            let idx_write_end = nbytes_rest + 1;
            while (self.idx_first < idx_write_end || self.idx_write <= self.idx_first) && self.nitems > 0 {
                let mut idx = self.idx_first;
                while self.buf[idx] != 0x00 {
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                }
                idx += 1;
                if idx >= self.buf.len() { idx = 0; }
                self.idx_first = idx;
                self.nitems -= 1;
            }
            self.buf[self.idx_write..self.idx_write + nbytes_copy].copy_from_slice(&src[..nbytes_copy]);
            self.buf[0..nbytes_rest].copy_from_slice(&src[nbytes_copy..]);
            self.buf[nbytes_rest] = 0x00;
            self.idx_last = self.idx_write;
            if self.nitems == 0 { self.idx_first = self.idx_write; }
            self.idx_write = idx_write_end;
            self.nitems += 1;
        }
        if self.idx_write >= self.buf.len() { self.idx_write = 0; }
        Ok(())
    }

    pub fn print(&self) {
        const NCOLS: usize = 16;
        println!("first:0x{:04x} last:0x{:04x} write:0x{:04x} count:{}",
            self.idx_first, self.idx_last, self.idx_write, self.count());
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

    pub fn iter(&self) -> Iterator<'_> {
        Iterator::new(self)
    }
}

pub struct Iterator<'a> {
    round_buf: &'a RoundBuffer<'a>,
    idx: usize,
    i_item: usize,
}

impl<'a> Iterator<'a> {
    pub fn new(round_buf: &'a RoundBuffer<'a>) -> Self {
        Self { round_buf, idx: round_buf.idx_first, i_item: 0 }
    }
}

impl<'a> core::iter::Iterator for Iterator<'a> {
    type Item = (&'a [u8], &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        if self.i_item == self.round_buf.count() { return None; }
        let idx_start = self.idx;
        let mut round_up = false;
        let buf = &self.round_buf.buf;
        let mut idx = idx_start;
        while buf[idx] != 0x00 {
            idx += 1;
            if idx >= buf.len() { idx = 0; round_up = true; }
        }
        let idx_end = idx;
        idx += 1;
        if idx >= buf.len() { idx = 0; }
        self.idx = idx;
        self.i_item += 1;
        if idx_end == 0 {
            Some((&buf[idx_start..], &buf[0..0]))
        } else if round_up {
            Some((&buf[idx_start..], &buf[..idx_end]))
        } else {
            Some((&buf[idx_start..idx_end], &buf[0..0]))
        }
    }
}
