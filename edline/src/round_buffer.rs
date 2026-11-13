pub struct RoundBuffer<'a> {
    buf: &'a mut [u8],
    idx_top: usize,
    idx_bottom: usize,
    idx_write: usize,
    //idx_cursor: usize,
    buf_full: bool,
    n_items: usize,
}

impl<'a> RoundBuffer<'a> {
    pub const IDX_NONE: usize = usize::MAX;

    pub fn new(buf: &'a mut [u8]) -> Self {
        buf.fill(0x00);
        Self { buf, idx_top: 0, idx_bottom: 0, idx_write: 0, buf_full: false, n_items: 0 }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }

    pub fn is_empty(&self) -> bool { self.n_items == 0 }

    pub fn buf(&self) -> &[u8] { self.buf }

    pub fn idx_top(&self) -> usize { self.idx_top }

    pub fn idx_bottom(&self) -> usize { self.idx_bottom }

    pub fn idx_write(&self) -> usize { self.idx_write }

    pub fn count(&self) -> usize { self.n_items }

    pub fn push(&mut self, src: &[u8]) -> Result<(), ()> {
        if src.len() + 1 > self.capacity() { return Err(()); }
        //self.idx_cursor = Self::IDX_NONE;
        let idx_write_end = self.idx_write + src.len() + 1;
        if idx_write_end <= self.buf.len() {
            if self.buf_full {
                while self.idx_write <= self.idx_top && self.idx_top < idx_write_end && self.n_items > 0 {
                    let mut idx = self.idx_top;
                    while self.buf[idx] != 0x00 {
                        idx += 1;
                        if idx >= self.buf.len() { idx = 0; }
                    }
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                    self.idx_top = idx;
                    self.n_items -= 1;
                }
            }
            self.buf[self.idx_write..self.idx_write + src.len()].copy_from_slice(src);
            self.buf[idx_write_end - 1] = 0x00;
            self.idx_bottom = self.idx_write;
            if self.n_items == 0 { self.idx_top = self.idx_write; }
            self.idx_write = idx_write_end;
            self.n_items += 1;
        } else {
            self.buf_full = true;
            let nbytes_copy = self.buf.len() - self.idx_write;
            let nbytes_rest = src.len() - nbytes_copy;
            let idx_write_end = nbytes_rest + 1;
            while (self.idx_top < idx_write_end || self.idx_write <= self.idx_top) && self.n_items > 0 {
                let mut idx = self.idx_top;
                while self.buf[idx] != 0x00 {
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                }
                idx += 1;
                if idx >= self.buf.len() { idx = 0; }
                self.idx_top = idx;
                self.n_items -= 1;
            }
            self.buf[self.idx_write..self.idx_write + nbytes_copy].copy_from_slice(&src[..nbytes_copy]);
            self.buf[0..nbytes_rest].copy_from_slice(&src[nbytes_copy..]);
            self.buf[nbytes_rest] = 0x00;
            self.idx_bottom = self.idx_write;
            if self.n_items == 0 { self.idx_top = self.idx_write; }
            self.idx_write = idx_write_end;
            self.n_items += 1;
        }
        if self.idx_write >= self.buf.len() { self.idx_write = 0; }
        Ok(())
    }

    pub fn get_item_at(&self, index: usize) -> Option<(&[u8], &[u8], bool)> {
        if index == Self::IDX_NONE || self.is_empty() { return None; }
        self.get_item(index, index == self.idx_bottom)
    }

    pub fn move_index_forward(&self, index: &mut usize) -> bool {
        if *index == Self::IDX_NONE {
            if self.is_empty() { return false; }
            *index = self.idx_top;
        } else if *index != self.idx_bottom {
            *index = {
                let mut idx = *index;
                while self.buf[idx] != 0x00 {
                    idx = if idx + 1 < self.buf.len() { idx + 1 } else { 0 };
                    assert!(idx != *index, "Failed to find null terminator in buffer");
                }
                if idx + 1 < self.buf.len() { idx + 1 } else { 0 }
            };
        }
        true
    }

    pub fn move_index_backward(&self, index: &mut usize) -> bool {
        self.move_index_backward_n(index, 1)
    }

    pub fn move_index_backward_n(&self, index: &mut usize, n: usize) -> bool {
        if *index == Self::IDX_NONE {
            if self.is_empty() { return false; }
            *index = self.idx_bottom;
        } else {
            for _ in 0..n {
                if *index == self.idx_top { break; }
                *index = {
                    let mut idx = *index;
                    idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                    let mut idx_start;
                    loop {
                        idx_start = idx;
                        if idx_start == self.idx_top { break; }
                        idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                        if self.buf[idx] == 0x00 { break; }
                        assert!(idx != *index, "Failed to find null terminator in buffer");
                    }
                    idx_start
                };
            }
        }
        true
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
