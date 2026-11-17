#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttr {
    buf: [u8; Self::NBYTES],
    nbytes_cur: usize,
}

impl CharAttr {
    pub const NBYTES: usize = 4;

    pub fn new() -> Self {
        Self { buf: [0xffu8; Self::NBYTES], nbytes_cur: 0 }
    }

    pub fn clear(&mut self) {
        self.nbytes_cur = 0;
    }

    pub fn is_complete(&self) -> bool {
        self.nbytes_cur == self.buf.len()
    }

    pub fn push(&mut self, byte: u8) {
        if self.nbytes_cur < self.buf.len() {
            self.buf[self.nbytes_cur] = byte;
            self.nbytes_cur += 1;
        }
    }

    pub fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
        self.nbytes_cur = other.nbytes_cur;
    }
}
