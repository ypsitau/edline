//-----------------------------------------------------------------------------
// CharAttr
//-----------------------------------------------------------------------------
pub trait CharAttr: PartialEq {
    const NBYTES: usize;

    fn new() -> Self;

    fn default() -> Self;

    fn clear(&mut self);

    fn is_complete(&self) -> bool;

    fn push(&mut self, byte: u8);

    fn clone_from(&mut self, other: &Self);

    fn get_bytes(&self) -> &[u8];

    fn ansi_foreground(&self) -> u8;

    fn ansi_background(&self) -> u8;
}

//-----------------------------------------------------------------------------
// CharAttrConstColor
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttrConstColor;

impl CharAttr for CharAttrConstColor {
    const NBYTES: usize = 0;

    fn new() -> Self { Self }

    fn default() -> Self { Self }

    fn clear(&mut self) {}

    fn is_complete(&self) -> bool { true }

    fn push(&mut self, _byte: u8) {}

    fn clone_from(&mut self, _other: &Self) {}

    fn get_bytes(&self) -> &[u8] { &[] }

    fn ansi_foreground(&self) -> u8 { 0 }

    fn ansi_background(&self) -> u8 { 0 }
}

//-----------------------------------------------------------------------------
// CharAttr16Colors
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttr16Colors {
    buf: [u8; Self::NBYTES],
    nbytes_cur: usize,
}

impl CharAttr for CharAttr16Colors {
    const NBYTES: usize = 1;

    fn new() -> Self {
        Self { buf: [0x0fu8; Self::NBYTES], nbytes_cur: 0 }
    }

    fn default() -> Self {
        Self { buf: [0x0fu8; Self::NBYTES], nbytes_cur: Self::NBYTES }
    }

    fn clear(&mut self) {
        self.nbytes_cur = 0;
    }

    fn is_complete(&self) -> bool {
        self.nbytes_cur == self.buf.len()
    }

    fn push(&mut self, byte: u8) {
        if self.nbytes_cur < self.buf.len() {
            self.buf[self.nbytes_cur] = byte;
            self.nbytes_cur += 1;
        }
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
        self.nbytes_cur = other.nbytes_cur;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }
    fn ansi_foreground(&self) -> u8 {
        self.buf[0] & 0x0f
    }

    fn ansi_background(&self) -> u8 {
        (self.buf[0] >> 4) & 0x0f
    }
}

//-----------------------------------------------------------------------------
// CharAttr256Colors
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttr256Colors {
    buf: [u8; Self::NBYTES],
    nbytes_cur: usize,
}

impl CharAttr for CharAttr256Colors {
    const NBYTES: usize = 2;

    fn new() -> Self {
        Self { buf: [0x0fu8; Self::NBYTES], nbytes_cur: 0 }
    }

    fn default() -> Self {
        Self { buf: [0x0fu8; Self::NBYTES], nbytes_cur: Self::NBYTES }
    }

    fn clear(&mut self) {
        self.nbytes_cur = 0;
    }

    fn is_complete(&self) -> bool {
        self.nbytes_cur == self.buf.len()
    }

    fn push(&mut self, byte: u8) {
        if self.nbytes_cur < self.buf.len() {
            self.buf[self.nbytes_cur] = byte;
            self.nbytes_cur += 1;
        }
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
        self.nbytes_cur = other.nbytes_cur;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn ansi_foreground(&self) -> u8 {
        self.buf[0]
    }

    fn ansi_background(&self) -> u8 {
        self.buf[1]
    }
}
