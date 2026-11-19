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

    fn has_color(&self) -> bool;

    fn has_deco(&self) -> bool;

    fn is_valid(&self) -> bool;

    fn is_applicable(&self) -> bool { (self.has_color() || self.has_deco()) && self.is_valid() }

    fn put_byte(&mut self, ibyte: usize, byte: u8);

    fn clone_from(&mut self, other: &Self);

    fn get_bytes(&self) -> &[u8];

    fn get_color(&self) -> (Color, Color) { (self.get_color_fg(), self.get_color_bg()) }

    fn get_color_fg(&self) -> Color;

    fn get_color_bg(&self) -> Color;

    fn set_color(&mut self, fg: Color, bg: Color);

    fn set_color_fg(&mut self, fg: Color);

    fn set_color_bg(&mut self, bg: Color);

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

    fn is_changed_color_fg(&self, other: &Self) -> bool { self.get_color_fg() != other.get_color_fg() }

    fn is_changed_color_bg(&self, other: &Self) -> bool { self.get_color_bg() != other.get_color_bg() }

    fn is_changed_bold(&self, other: &Self) -> bool { self.is_bold() != other.is_bold() }

    fn is_changed_italic(&self, other: &Self) -> bool { self.is_italic() != other.is_italic() }

    fn is_changed_underlined(&self, other: &Self) -> bool { self.is_underlined() != other.is_underlined() }

    fn is_changed_strikethrough(&self, other: &Self) -> bool { self.is_strikethrough() != other.is_strikethrough() }
}

//-----------------------------------------------------------------------------
// CharAttrMonoColor
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttrMonoColor;

impl CharAttr for CharAttrMonoColor {
    const NBYTES: usize = 0;
    const INVALID: Self = Self;

    fn new() -> Self { Self }

    fn has_color(&self) -> bool { false }

    fn has_deco(&self) -> bool { false }

    fn is_valid(&self) -> bool { true }

    fn put_byte(&mut self, _ibyte: usize, _byte: u8) {}

    fn clone_from(&mut self, _other: &Self) {}

    fn get_bytes(&self) -> &[u8] { &[] }

    fn get_color_fg(&self) -> Color { Color::BLACK }

    fn get_color_bg(&self) -> Color { Color::WHITE }

    fn set_color(&mut self, _fg: Color, _bg: Color) {}

    fn set_color_fg(&mut self, _fg: Color) {}

    fn set_color_bg(&mut self, _bg: Color) {}

    fn get_deco(&self) -> u8 { DECO_DEFAULT }

    fn set_deco(&mut self, _deco: u8) {}
}

//-----------------------------------------------------------------------------
// CharAttrMonoColorDeco
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CharAttrMonoColorDeco {
    buf: [u8; Self::NBYTES],
}

impl CharAttr for CharAttrMonoColorDeco {
    const NBYTES: usize = 1;
    const INVALID: Self = Self { buf: [0x00u8; Self::NBYTES] };

    fn new() -> Self { Self { buf: [DECO_DEFAULT] } }

    fn has_color(&self) -> bool { false }

    fn has_deco(&self) -> bool { true }

    fn is_valid(&self) -> bool { true }

    fn put_byte(&mut self, _ibyte: usize, _byte: u8) {}

    fn clone_from(&mut self, _other: &Self) {}

    fn get_bytes(&self) -> &[u8] { &[] }

    fn get_color_fg(&self) -> Color { Color::BLACK }

    fn get_color_bg(&self) -> Color { Color::WHITE }

    fn set_color(&mut self, _fg: Color, _bg: Color) {}

    fn set_color_fg(&mut self, _fg: Color) {}

    fn set_color_bg(&mut self, _bg: Color) {}

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
        Self { buf: [fg.get_code() | (bg.get_code() << 4)] }
    }

    fn has_color(&self) -> bool { true }

    fn has_deco(&self) -> bool { false }

    fn is_valid(&self) -> bool { *self != Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0] & 0x0f) }

    fn get_color_bg(&self) -> Color { Color::from_code((self.buf[0] >> 4) & 0x0f) }

    fn set_color(&mut self, fg: Color, bg: Color) {
        let code = (bg.get_code() << 4) | (fg.get_code() & 0x0f);
        // CharAttr is not allowed to contain the code 0x00.
        if code != 0x00 { self.buf[0] = code; }
    }

    fn set_color_fg(&mut self, fg: Color) {
        self.set_color(fg, self.get_color_bg());
    }

    fn set_color_bg(&mut self, bg: Color) {
        self.set_color(self.get_color_fg(), bg);
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
        Self { buf: [fg.get_code() | (bg.get_code() << 4), DECO_DEFAULT] }
    }

    fn has_color(&self) -> bool { true }

    fn has_deco(&self) -> bool { true }

    fn is_valid(&self) -> bool { *self != Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }


    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0] & 0x0f) }

    fn get_color_bg(&self) -> Color { Color::from_code((self.buf[0] >> 4) & 0x0f) }

    fn set_color(&mut self, fg: Color, bg: Color) {
        let code = (bg.get_code() << 4) | (fg.get_code() & 0x0f);
        // CharAttr is not allowed to contain the code 0x00.
        if code != 0x00 { self.buf[0] = code; }
    }

    fn set_color_fg(&mut self, fg: Color) {
        self.set_color(fg, self.get_color_bg());
    }

    fn set_color_bg(&mut self, bg: Color) {
        self.set_color(self.get_color_fg(), bg);
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

    fn has_color(&self) -> bool { true }

    fn has_deco(&self) -> bool { false }

    fn is_valid(&self) -> bool { *self != Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0]) }

    fn get_color_bg(&self) -> Color { Color::from_code(self.buf[1]) }

    fn set_color(&mut self, fg: Color, bg: Color) {
        // Convert standard colors to RGB cube before storing their codes.
        // This avoids storing 0x00 code for standard colors.
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
    }

    fn set_color_fg(&mut self, fg: Color) {
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
    }

    fn set_color_bg(&mut self, bg: Color) {
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

    fn has_color(&self) -> bool { true }

    fn has_deco(&self) -> bool { true }

    fn is_valid(&self) -> bool { *self != Self::INVALID }

    fn put_byte(&mut self, ibyte: usize, byte: u8) {
        self.buf[ibyte] = byte;
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf = other.buf;
    }

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0]) }

    fn get_color_bg(&self) -> Color { Color::from_code(self.buf[1]) }

    fn set_color(&mut self, fg: Color, bg: Color) {
        // Convert standard colors to RGB cube before storing their codes.
        // This avoids storing 0x00 code for standard colors.
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
    }

    fn set_color_fg(&mut self, fg: Color) {
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
    }

    fn set_color_bg(&mut self, bg: Color) {
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
    }

    fn get_deco(&self) -> u8 { self.buf[2] }

    fn set_deco(&mut self, deco: u8) { self.buf[2] = deco; }
}
