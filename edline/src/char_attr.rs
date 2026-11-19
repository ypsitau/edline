use crate::color::Color;

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

    fn set_ansi(&mut self, fg: Color, bg: Color);

    fn get_ansi(&self) -> (Color, Color);
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

    fn set_ansi(&mut self, _fg: Color, _bg: Color) {}

    fn get_ansi(&self) -> (Color, Color) { (Color::BLACK, Color::WHITE) }
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

    fn set_ansi(&mut self, fg: Color, bg: Color) {
        let code = (bg.get_code() << 4) | (fg.get_code() & 0x0f);
        // CharAttr is not allowed to contain the code 0x00.
        if code != 0x00 { self.buf[0] = code; }
    }

    fn get_ansi(&self) -> (Color, Color) {
        let fg = Color::from_code(self.buf[0] & 0x0f);
        let bg = Color::from_code((self.buf[0] >> 4) & 0x0f);
        (fg, bg)
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

    fn set_ansi(&mut self, fg: Color, bg: Color) {
        // Convert standard colors to RGB cube before storing their codes.
        // This avoids storing 0x00 code for standard colors.
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
    }

    fn get_ansi(&self) -> (Color, Color) {
        let fg = Color::from_code(self.buf[0]);
        let bg = Color::from_code(self.buf[1]);
        (fg, bg)
    }
}
