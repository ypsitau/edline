pub struct RoundBuffer<'a> {
    buf: &'a mut [u8],
    idx_top: usize,
    idx_bottom: usize,
    idx_write: usize,
    idx_cursor: usize,
    buf_full: bool,
    nitems: usize,
}

impl<'a> RoundBuffer<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, idx_top: 0, idx_bottom: 0, idx_write: 0, idx_cursor: usize::MAX, buf_full: false, nitems: 0 }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }
    pub fn is_empty(&self) -> bool { self.nitems == 0 }
    pub fn count(&self) -> usize { self.nitems }
    pub fn add(&mut self, src: &[u8]) -> Result<(), ()> {
        if src.len() + 1 > self.capacity() { return Err(()); }
        self.idx_cursor = usize::MAX;
        let idx_write_end = self.idx_write + src.len() + 1;
        if idx_write_end <= self.buf.len() {
            if self.buf_full {
                while self.idx_write <= self.idx_top && self.idx_top < idx_write_end && self.nitems > 0 {
                    let mut idx = self.idx_top;
                    while self.buf[idx] != 0x00 {
                        idx += 1;
                        if idx >= self.buf.len() { idx = 0; }
                    }
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                    self.idx_top = idx;
                    self.nitems -= 1;
                }
            }
            self.buf[self.idx_write..idx_write_end - 1].copy_from_slice(src);
            self.buf[idx_write_end - 1] = 0x00;
            self.idx_bottom = self.idx_write;
            if self.nitems == 0 { self.idx_top = self.idx_write; }
            self.idx_write = idx_write_end;
            self.nitems += 1;
        } else {
            self.buf_full = true;
            let nbytes_copy = self.buf.len() - self.idx_write;
            let nbytes_rest = src.len() - nbytes_copy;
            let idx_write_end = nbytes_rest + 1;
            while (self.idx_top < idx_write_end || self.idx_write <= self.idx_top) && self.nitems > 0 {
                let mut idx = self.idx_top;
                while self.buf[idx] != 0x00 {
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                }
                idx += 1;
                if idx >= self.buf.len() { idx = 0; }
                self.idx_top = idx;
                self.nitems -= 1;
            }
            self.buf[self.idx_write..self.idx_write + nbytes_copy].copy_from_slice(&src[..nbytes_copy]);
            self.buf[0..nbytes_rest].copy_from_slice(&src[nbytes_copy..]);
            self.buf[nbytes_rest] = 0x00;
            self.idx_bottom = self.idx_write;
            if self.nitems == 0 { self.idx_top = self.idx_write; }
            self.idx_write = idx_write_end;
            self.nitems += 1;
        }
        if self.idx_write >= self.buf.len() { self.idx_write = 0; }
        Ok(())
    }

    pub fn reset_cursor(&mut self) {
        self.idx_cursor = usize::MAX;
    }

    pub fn move_cursor_forward(&mut self) -> Option<(&[u8], &[u8], bool)> {
        self.move_cursor(false)
    }

    pub fn move_cursor_backward(&mut self) -> Option<(&[u8], &[u8], bool)> {
        self.move_cursor(true)
    }

    pub fn move_cursor(&mut self, backward: bool) -> Option<(&[u8], &[u8], bool)> {
        if self.idx_cursor == usize::MAX {
            if self.is_empty() { return None; }
            self.idx_cursor = if backward { self.idx_bottom } else { self.idx_top };
        }
        let idx_start = self.idx_cursor;
        let idx_end = {
            let mut idx = idx_start;
            while self.buf[idx] != 0x00 {
                idx += 1;
                if idx >= self.buf.len() { idx = 0; }
                assert!(idx != idx_start, "Failed to find null terminator in buffer");
            }
            idx
        };
        let mut last_item = false;
        if backward {
            last_item = (idx_start == self.idx_top);
            if !last_item {
                let mut idx = idx_start;
                idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                loop {
                    self.idx_cursor = idx;
                    idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                    if self.buf[idx] == 0x00 { break; }
                }
            }
        } else {
            last_item = (idx_start == self.idx_bottom);
            if !last_item {
                let mut idx = idx_end + 1;
                if idx >= self.buf.len() { idx = 0; }
                self.idx_cursor = idx;
            }
        }
        if idx_end == 0 {
            Some((&self.buf[idx_start..], &self.buf[0..0], last_item))
        } else if idx_end < idx_start {
            Some((&self.buf[idx_start..], &self.buf[..idx_end], last_item))
        } else {
            Some((&self.buf[idx_start..idx_end], &self.buf[0..0], last_item))
        }
    }

    #[cfg(feature = "std")]
    pub fn print_buf(&self) {
        const NCOLS: usize = 16;
        println!("first:0x{:04x} last:0x{:04x} write:0x{:04x} count:{}",
            self.idx_top, self.idx_bottom, self.idx_write, self.count());
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
                if idx == self.idx_top { print!("F"); n += 1;}
                if idx == self.idx_bottom { print!("L"); n += 1; }
                if idx == self.idx_write { print!("W"); n += 1; }
                for _ in n..3 { print!(" "); }
                idx += 1;
            }
            println!();
        }
    }

    #[cfg(feature = "std")]
    pub fn print_items_forward(&mut self) {
        self.reset_cursor();
        while let Some((first, second, last_item)) = self.move_cursor_forward() {
            println!("(\"{}\", \"{}\")", str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
            if last_item { break; }
        }
    }

    #[cfg(feature = "std")]
    pub fn print_items_backward(&mut self) {
        self.reset_cursor();
        while let Some((first, second, last_item)) = self.move_cursor_backward() {
            println!("(\"{}\", \"{}\")", str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
            if last_item { break; }
        }
    }
}

/*
pub struct Iterator<'a> {
    round_buf: &'a RoundBuffer<'a>,
    idx: usize,
    iitem: usize,
}

impl<'a> Iterator<'a> {
    pub fn new(round_buf: &'a RoundBuffer<'a>) -> Self {
        Self { round_buf, idx: round_buf.idx_first, iitem: 0 }
    }
}

impl<'a> core::iter::Iterator for Iterator<'a> {
    type Item = (&'a [u8], &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        if self.iitem == self.round_buf.count() { return None; }
        let idx_start = self.idx;
        let buf = &self.round_buf.buf;
        let mut idx = idx_start;
        while buf[idx] != 0x00 {
            idx += 1;
            if idx >= buf.len() { idx = 0; }
            assert!(idx != idx_start, "Failed to find null terminator in buffer");
        }
        let idx_end = idx;
        idx += 1;
        if idx >= buf.len() { idx = 0; }
        
        self.idx = idx;
        
        self.iitem += 1;
        if idx_end == 0 {
            Some((&buf[idx_start..], &buf[0..0]))
        } else if idx_end < idx_start {
            Some((&buf[idx_start..], &buf[..idx_end]))
        } else {
            Some((&buf[idx_start..idx_end], &buf[0..0]))
        }
    }
}
*/
