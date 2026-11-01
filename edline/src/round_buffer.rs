pub struct RoundBuffer<'a> {
    buf: &'a mut [u8],
    idx_read: usize,
    idx_head: usize,
    idx_write: usize,
}

impl<'a> RoundBuffer<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, idx_read: 0, idx_head: 0, idx_write: 0 }
    }
    pub fn capacity(&self) -> usize { self.buf.len() }
    pub fn write(&mut self, src: &[u8]) -> Result<(), ()> {
        if self.idx_write >= self.buf.len() { self.idx_write = 0; }
        let idx_write_end = self.idx_write + src.len();
        if idx_write_end <= self.buf.len() {
            let mut round_up = false;
            while self.idx_head < self.idx_write && !round_up {
                self.idx_head += 1;
                if self.idx_head >= self.buf.len() { self.idx_head = 0; }
                while self.buf[self.idx_head] != 0 {
                    self.idx_head += 1;
                    if self.idx_head >= self.buf.len() {
                        self.idx_head = 0;
                        round_up = true;
                    }
                }
            }
            self.buf[self.idx_write..idx_write_end].copy_from_slice(src);
            self.idx_write += src.len();
        } else {
            let nbytes_copy = self.buf.len() - self.idx_write;
            self.buf[self.idx_write..].copy_from_slice(&src[..nbytes_copy]);
            let nbytes_rest = src.len() - nbytes_copy;
            self.buf[..nbytes_rest].copy_from_slice(&src[nbytes_copy..]);
            self.idx_write = nbytes_rest;
        }
        if self.idx_write >= self.buf.len() { self.idx_write = 0; }
        self.buf[self.idx_write] = 0;
        self.idx_write += 1;
        Ok(())
    }
    pub fn read(&mut self, dst: &mut [u8]) -> Result<(), ()> {
        let mut idx = 0;
        let mut bytes_read = 0;
        while bytes_read < dst.len() {
            let data = self.buf[self.idx_read];
            if data == 0 { break; }
            dst[idx] = data;
            idx += 1;
            self.idx_read += 1;
            bytes_read += 1;
            if self.idx_read >= self.buf.len() { self.idx_read = 0; }
        }
        Ok(())
    }
}
