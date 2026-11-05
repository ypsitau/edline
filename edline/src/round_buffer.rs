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
    const IDX_NONE: usize = usize::MAX;

    pub fn new(buf: &'a mut [u8]) -> Self {
        buf.fill(0x00);
        Self { buf, idx_top: 0, idx_bottom: 0, idx_write: 0, idx_cursor: Self::IDX_NONE, buf_full: false, nitems: 0 }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }

    pub fn is_empty(&self) -> bool { self.nitems == 0 }

    pub fn buf(&self) -> &[u8] { self.buf }

    pub fn idx_top(&self) -> usize { self.idx_top }

    pub fn idx_bottom(&self) -> usize { self.idx_bottom }

    pub fn idx_write(&self) -> usize { self.idx_write }

    pub fn count(&self) -> usize { self.nitems }

    pub fn push(&mut self, src: &[u8]) -> Result<(), ()> {
        if src.len() + 1 > self.capacity() { return Err(()); }
        self.idx_cursor = Self::IDX_NONE;
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
        self.idx_cursor = Self::IDX_NONE;
    }

    pub fn move_cursor_forward(&mut self) -> Option<(&[u8], &[u8], bool)> {
        if self.idx_cursor == Self::IDX_NONE {
            if self.is_empty() { return None; }
            self.idx_cursor = self.idx_top;
        } else if self.idx_cursor != self.idx_bottom {
            self.idx_cursor = {
                let mut idx = self.idx_cursor;
                while self.buf[idx] != 0x00 {
                    idx = if idx + 1 < self.buf.len() { idx + 1 } else { 0 };
                    assert!(idx != self.idx_cursor, "Failed to find null terminator in buffer");
                }
                if idx + 1 < self.buf.len() { idx + 1 } else { 0 }
            };
        }
        self.get_item(self.idx_cursor, self.idx_cursor == self.idx_bottom)
    }

    pub fn move_cursor_backward(&mut self) -> Option<(&[u8], &[u8], bool)> {
        if self.idx_cursor == Self::IDX_NONE {
            if self.is_empty() { return None; }
            self.idx_cursor = self.idx_bottom;
        } else if self.idx_cursor != self.idx_top {
            self.idx_cursor = {
                let mut idx = self.idx_cursor;
                idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                let mut idx_start;
                loop {
                    idx_start = idx;
                    if idx_start == self.idx_top { break; }
                    idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                    if self.buf[idx] == 0x00 { break; }
                    assert!(idx != self.idx_cursor, "Failed to find null terminator in buffer");
                }
                idx_start
            };
        }
        self.get_item(self.idx_cursor, self.idx_cursor == self.idx_top)
    }

    fn get_item(&self, idx_item: usize, last_item: bool) -> Option<(&[u8], &[u8], bool)> {
        let idx_end = {
            let mut idx = idx_item;
            while self.buf[idx] != 0x00 {
                idx = if idx + 1 < self.buf.len() { idx + 1 } else { 0 };
                assert!(idx != idx_item, "Failed to find null terminator in buffer");
            }
            idx
        };
        if idx_end == 0 {
            Some((&self.buf[idx_item..], &self.buf[0..0], last_item))
        } else if idx_end < idx_item {
            Some((&self.buf[idx_item..], &self.buf[..idx_end], last_item))
        } else {
            Some((&self.buf[idx_item..idx_end], &self.buf[0..0], last_item))
        }
    }
}
