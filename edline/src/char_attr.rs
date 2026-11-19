use crate::color::Color;

const DECO_DEFAULT: u8 = 0x80;
const DECO_BOLD: u8 = 1 << 0;
const DECO_ITALIC: u8 = 1 << 1;
const DECO_UNDERLINED: u8 = 1 << 2;
const DECO_STRIKETHROUGH: u8 = 1 << 3;
//-----------------------------------------------------------------------------
// CharAttr
//-----------------------------------------------------------------------------
pub trait CharAttr: PartialEq {
    const NBYTES: usize;
    const INVALID: Self;

    fn new() -> Self;

    fn is_const(&self) -> bool;

    fn is_invalid(&self) -> bool;

    fn put_byte(&mut self, ibyte: usize, byte: u8);

    fn clone_from(&mut self, other: &Self);

    fn get_bytes(&self) -> &[u8];

    fn get_color(&self) -> (Color, Color);

    fn set_color(&mut self, fg: Color, bg: Color);

    fn get_deco(&self) -> u8;

    fn set_deco(&mut self, _deco: u8);

    fn is_bold(&self) -> bool { self.get_deco() & DECO_BOLD != 0 }

    fn is_italic(&self) -> bool { self.get_deco() & DECO_ITALIC != 0 }

    fn is_underlined(&self) -> bool { self.get_deco() & DECO_UNDERLINED != 0 }

    fn is_strikethrough(&self) -> bool { self.get_deco() & DECO_STRIKETHROUGH != 0 }

    fn set_bold(&mut self) { self.set_deco(self.get_deco() | DECO_BOLD); }

    fn clear_bold(&mut self) { self.set_deco(self.get_deco() & !DECO_BOLD); }

    fn set_italic(&mut self) { self.set_deco(self.get_deco() | DECO_ITALIC); }

    fn clear_italic(&mut self) { self.set_deco(self.get_deco() & !DECO_ITALIC); }

    fn set_underlined(&mut self) { self.set_deco(self.get_deco() | DECO_UNDERLINED); }

    fn clear_underlined(&mut self) { self.set_deco(self.get_deco() & !DECO_UNDERLINED); }

    fn set_strikethrough(&mut self) { self.set_deco(self.get_deco() | DECO_STRIKETHROUGH); }

    fn clear_strikethrough(&mut self) { self.set_deco(self.get_deco() & !DECO_STRIKETHROUGH); }
}

//-----------------------------------------------------------------------------
// CharAttrConstColor
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttrConstColor;

impl CharAttr for CharAttrConstColor {
    const NBYTES: usize = 0;
    const INVALID: Self = Self;

    fn new() -> Self { Self }

    fn is_const(&self) -> bool { true }

    fn is_invalid(&self) -> bool { false }

    fn put_byte(&mut self, _ibyte: usize, _byte: u8) {}

    fn clone_from(&mut self, _other: &Self) {}

    fn get_bytes(&self) -> &[u8] { &[] }

    fn get_color(&self) -> (Color, Color) { (Color::BLACK, Color::WHITE) }

    fn set_color(&mut self, _fg: Color, _bg: Color) {}

    fn get_deco(&self) -> u8 { DECO_DEFAULT }

    fn set_deco(&mut self, _deco: u8) {}
}

//-----------------------------------------------------------------------------
// CharAttrConstColorDeco
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttrConstColorDeco {
    buf: [u8; Self::NBYTES],
}

impl CharAttr for CharAttrConstColorDeco {
    const NBYTES: usize = 1;
    const INVALID: Self = Self { buf: [0x00u8; Self::NBYTES] };

    fn new() -> Self { Self { buf: [DECO_DEFAULT] } }

    fn is_const(&self) -> bool { true }

    fn is_invalid(&self) -> bool { false }

    fn put_byte(&mut self, _ibyte: usize, _byte: u8) {}

    fn clone_from(&mut self, _other: &Self) {}

    fn get_bytes(&self) -> &[u8] { &[] }

    fn get_color(&self) -> (Color, Color) { (Color::BLACK, Color::WHITE) }

    fn set_color(&mut self, _fg: Color, _bg: Color) {}

    fn get_deco(&self) -> u8 { self.buf[0] }

    fn set_deco(&mut self, deco: u8) { self.buf[0] = deco; }
}

//-----------------------------------------------------------------------------
// CharAttr16Colors
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttr16Colors {
    buf: [u8; Self::NBYTES],
}

impl CharAttr for CharAttr16Colors {
    const NBYTES: usize = 1;
    const INVALID: Self = Self { buf: [0x00u8; Self::NBYTES] };

    fn new() -> Self {
        let (fg, bg) = (Color::BLACK, Color::WHITE);
        Self { buf: [fg.get_code() | (bg.get_code() & 0x0f) << 4] }
    }

    fn is_const(&self) -> bool {
        false
    }

    fn is_invalid(&self) -> bool { *self == Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color(&self) -> (Color, Color) {
        let fg = Color::from_code(self.buf[0] & 0x0f);
        let bg = Color::from_code((self.buf[0] >> 4) & 0x0f);
        (fg, bg)
    }

    fn set_color(&mut self, fg: Color, bg: Color) {
        let code = (bg.get_code() << 4) | (fg.get_code() & 0x0f);
        // CharAttr is not allowed to contain the code 0x00.
        if code != 0x00 { self.buf[0] = code; }
    }

    fn get_deco(&self) -> u8 { DECO_DEFAULT }

    fn set_deco(&mut self, _deco: u8) {}
}

//-----------------------------------------------------------------------------
// CharAttr16ColorsDeco
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttr16ColorsDeco {
    buf: [u8; Self::NBYTES],
}

impl CharAttr for CharAttr16ColorsDeco {
    const NBYTES: usize = 2;
    const INVALID: Self = Self { buf: [0x00u8; Self::NBYTES] };

    fn new() -> Self {
        let (fg, bg) = (Color::BLACK, Color::WHITE);
        Self { buf: [fg.get_code() | (bg.get_code() & 0x0f) << 4, DECO_DEFAULT] }
    }

    fn is_const(&self) -> bool {
        false
    }

    fn is_invalid(&self) -> bool { *self == Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color(&self) -> (Color, Color) {
        let fg = Color::from_code(self.buf[0] & 0x0f);
        let bg = Color::from_code((self.buf[0] >> 4) & 0x0f);
        (fg, bg)
    }

    fn set_color(&mut self, fg: Color, bg: Color) {
        let code = (bg.get_code() << 4) | (fg.get_code() & 0x0f);
        // CharAttr is not allowed to contain the code 0x00.
        if code != 0x00 { self.buf[0] = code; }
    }

    fn get_deco(&self) -> u8 { self.buf[1] }

    fn set_deco(&mut self, deco: u8) { self.buf[1] = deco; }
}

//-----------------------------------------------------------------------------
// CharAttr256Colors
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttr256Colors {
    buf: [u8; Self::NBYTES],
}

impl CharAttr for CharAttr256Colors {
    const NBYTES: usize = 2;
    const INVALID: Self = Self { buf: [0x00u8; Self::NBYTES] };

    fn new() -> Self {
        let (fg, bg) = (Color::from_rgb(0, 0, 0), Color::from_rgb(255, 255, 255));
        Self { buf: [fg.get_code(), bg.get_code()] }
    }

    fn is_const(&self) -> bool {
        false
    }

    fn is_invalid(&self) -> bool { *self == Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color(&self) -> (Color, Color) {
        let fg = Color::from_code(self.buf[0]);
        let bg = Color::from_code(self.buf[1]);
        (fg, bg)
    }

    fn set_color(&mut self, fg: Color, bg: Color) {
        // Convert standard colors to RGB cube before storing their codes.
        // This avoids storing 0x00 code for standard colors.
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
    }

    fn get_deco(&self) -> u8 { DECO_DEFAULT }

    fn set_deco(&mut self, _deco: u8) {}
}

//-----------------------------------------------------------------------------
// CharAttr256ColorsDeco
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttr256ColorsDeco {
    buf: [u8; Self::NBYTES],
}

impl CharAttr for CharAttr256ColorsDeco {
    const NBYTES: usize = 3;
    const INVALID: Self = Self { buf: [0x00u8; Self::NBYTES] };

    fn new() -> Self {
        let (fg, bg) = (Color::from_rgb(0, 0, 0), Color::from_rgb(255, 255, 255));
        Self { buf: [fg.get_code(), bg.get_code(), DECO_DEFAULT] }
    }

    fn is_const(&self) -> bool {
        false
    }

    fn is_invalid(&self) -> bool { *self == Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color(&self) -> (Color, Color) {
        let fg = Color::from_code(self.buf[0]);
        let bg = Color::from_code(self.buf[1]);
        (fg, bg)
    }

    fn set_color(&mut self, fg: Color, bg: Color) {
        // Convert standard colors to RGB cube before storing their codes.
        // This avoids storing 0x00 code for standard colors.
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
    }

    fn get_deco(&self) -> u8 { self.buf[2] }

    fn set_deco(&mut self, deco: u8) { self.buf[2] = deco; }
}
