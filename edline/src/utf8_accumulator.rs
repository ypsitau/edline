//-----------------------------------------------------------------------------
// Utf8Accumulator
//-----------------------------------------------------------------------------
pub struct Utf8Accumulator {
    buf: [u8; 8],
    idx: u8,
    idx_end: u8,
}

impl Utf8Accumulator {
    pub fn new() -> Self {
        Self { buf: [0; 8], idx: 0, idx_end: 0 }
    }

    pub fn reset(&mut self) {
        self.idx = 0;
        self.idx_end = 0;
    }

    pub fn push(&mut self, byte: u8) -> bool {
        if self.idx == 0 {
            self.buf[0] = byte;
            self.idx = 1;
            if byte & 0x80 == 0x00 {
                self.idx_end = 1;
                return true;
            } else if byte & 0xe0 == 0xc0 {
                self.idx_end = 2;
            } else if byte & 0xf0 == 0xe0 {
                self.idx_end = 3;
            } else if byte & 0xf8 == 0xf0 {
                self.idx_end = 4;
            } else {
                self.reset();
            };
        } else if byte & 0xc0 == 0x80 {
            self.buf[self.idx as usize] = byte;
            self.idx += 1;
            return self.idx == self.idx_end;
        } else {
            self.reset();
        }
        false
    }

    pub fn get_result(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.idx_end as usize]) }
    }
}
