use crate::color::Color;
use core::marker::PhantomData;

const DECO_DEFAULT: u8          = 0x80;
const DECO_BOLD: u8             = 1 << 0;
const DECO_ITALIC: u8           = 1 << 1;
const DECO_UNDERLINED: u8       = 1 << 2;
const DECO_STRIKETHROUGH: u8    = 1 << 3;

//-----------------------------------------------------------------------------
// CharAttr
//-----------------------------------------------------------------------------
pub trait CharAttr: Sized + Copy + PartialEq {
    const NBYTES: usize;
    const INVALID: Self;

    fn new() -> Self;

    fn has_color(&self) -> bool;

    fn has_deco(&self) -> bool;

    fn is_valid(&self) -> bool;

    fn put_byte(&mut self, ibyte: usize, byte: u8);

    fn get_bytes(&self) -> &[u8];

    fn get_color(&self) -> (Color, Color) { (self.get_color_fg(), self.get_color_bg()) }

    fn get_color_fg(&self) -> Color;

    fn get_color_bg(&self) -> Color;

    fn set_color(self, fg: Color, bg: Color) -> Self;

    fn set_color_fg(self, fg: Color) -> Self;

    fn set_color_bg(self, bg: Color) -> Self;

    fn get_deco(&self) -> u8;

    fn set_deco(self, _deco: u8) -> Self;

    fn is_bold(&self) -> bool { self.get_deco() & DECO_BOLD != 0 }

    fn is_italic(&self) -> bool { self.get_deco() & DECO_ITALIC != 0 }

    fn is_underlined(&self) -> bool { self.get_deco() & DECO_UNDERLINED != 0 }

    fn is_strikethrough(&self) -> bool { self.get_deco() & DECO_STRIKETHROUGH != 0 }

    fn set_bold(self) -> Self {
        let deco = self.get_deco() | DECO_BOLD;
        self.set_deco(deco)
    }

    fn clear_bold(self) -> Self {
        let deco = self.get_deco() & !DECO_BOLD;
        self.set_deco(deco)
    }

    fn set_italic(self) -> Self {
        let deco = self.get_deco() | DECO_ITALIC;
        self.set_deco(deco)
    }

    fn clear_italic(self) -> Self {
        let deco = self.get_deco() & !DECO_ITALIC;
        self.set_deco(deco)
    }

    fn set_underlined(self) -> Self {
        let deco = self.get_deco() | DECO_UNDERLINED;
        self.set_deco(deco)
    }

    fn clear_underlined(self) -> Self {
        let deco = self.get_deco() & !DECO_UNDERLINED;
        self.set_deco(deco)
    }

    fn set_strikethrough(self) -> Self {
        let deco = self.get_deco() | DECO_STRIKETHROUGH;
        self.set_deco(deco)
    }

    fn clear_strikethrough(self) -> Self {
        let deco = self.get_deco() & !DECO_STRIKETHROUGH;
        self.set_deco(deco)
    }

    fn is_changed_color_fg(&self, other: &Self) -> bool { self.get_color_fg() != other.get_color_fg() }

    fn is_changed_color_bg(&self, other: &Self) -> bool { self.get_color_bg() != other.get_color_bg() }

    fn is_changed_bold(&self, other: &Self) -> bool { self.is_bold() != other.is_bold() }

    fn is_changed_italic(&self, other: &Self) -> bool { self.is_italic() != other.is_italic() }

    fn is_changed_underlined(&self, other: &Self) -> bool { self.is_underlined() != other.is_underlined() }

    fn is_changed_strikethrough(&self, other: &Self) -> bool { self.is_strikethrough() != other.is_strikethrough() }
}

pub trait CharAttrTag {
    type CharAttr: CharAttr;
}

impl<T: CharAttr> CharAttrTag for PhantomData<T> {
    type CharAttr = T;
}

/// Tags for selecting character attribute types.
pub mod tag {
    macro_rules! char_attr_tags {
        ($($char_attr:ident),+ $(,)?) => {
            $(
                pub type $char_attr = core::marker::PhantomData<super::$char_attr>;

                #[allow(non_upper_case_globals)]
                pub const $char_attr: $char_attr = core::marker::PhantomData;
            )+
        };
    }
    char_attr_tags! {
        CharAttrMonoColor,
        CharAttrMonoColorDeco,
        CharAttr16Colors,
        CharAttr16ColorsDeco,
        CharAttr256Colors,
        CharAttr256ColorsDeco,
    }
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

    fn get_bytes(&self) -> &[u8] { &[] }

    fn get_color_fg(&self) -> Color { Color::BLACK }

    fn get_color_bg(&self) -> Color { Color::WHITE }

    fn set_color(self, _fg: Color, _bg: Color) -> Self { self }

    fn set_color_fg(self, _fg: Color) -> Self { self }

    fn set_color_bg(self, _bg: Color) -> Self { self }

    fn get_deco(&self) -> u8 { DECO_DEFAULT }

    fn set_deco(self, _deco: u8) -> Self { self }
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

    fn get_bytes(&self) -> &[u8] { &[] }

    fn get_color_fg(&self) -> Color { Color::BLACK }

    fn get_color_bg(&self) -> Color { Color::WHITE }

    fn set_color(self, _fg: Color, _bg: Color) -> Self { self }

    fn set_color_fg(self, _fg: Color) -> Self { self }

    fn set_color_bg(self, _bg: Color) -> Self { self }

    fn get_deco(&self) -> u8 { self.buf[0] }

    fn set_deco(mut self, deco: u8) -> Self {
        self.buf[0] = deco;
        self
    }
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

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0] & 0x0f) }

    fn get_color_bg(&self) -> Color { Color::from_code((self.buf[0] >> 4) & 0x0f) }

    fn set_color(mut self, fg: Color, bg: Color) -> Self {
        let code = (bg.get_code() << 4) | (fg.get_code() & 0x0f);
        // CharAttr is not allowed to contain the code 0x00.
        if code != 0x00 { self.buf[0] = code; }
        self
    }

    fn set_color_fg(self, fg: Color) -> Self {
        let bg = self.get_color_bg();
        self.set_color(fg, bg)
    }

    fn set_color_bg(self, bg: Color) -> Self {
        let fg = self.get_color_fg();
        self.set_color(fg, bg)
    }

    fn get_deco(&self) -> u8 { DECO_DEFAULT }

    fn set_deco(self, _deco: u8) -> Self { self }
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

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0] & 0x0f) }

    fn get_color_bg(&self) -> Color { Color::from_code((self.buf[0] >> 4) & 0x0f) }

    fn set_color(mut self, fg: Color, bg: Color) -> Self {
        let code = (bg.get_code() << 4) | (fg.get_code() & 0x0f);
        // CharAttr is not allowed to contain the code 0x00.
        if code != 0x00 { self.buf[0] = code; }
        self
    }

    fn set_color_fg(self, fg: Color) -> Self {
        let bg = self.get_color_bg();
        self.set_color(fg, bg)
    }

    fn set_color_bg(self, bg: Color) -> Self {
        let fg = self.get_color_fg();
        self.set_color(fg, bg)
    }

    fn get_deco(&self) -> u8 { self.buf[1] }

    fn set_deco(mut self, deco: u8) -> Self {
        self.buf[1] = deco;
        self
    }
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

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0]) }

    fn get_color_bg(&self) -> Color { Color::from_code(self.buf[1]) }

    fn set_color(mut self, fg: Color, bg: Color) -> Self {
        // Convert standard colors to RGB cube before storing their codes.
        // This avoids storing 0x00 code for standard colors.
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
        self
    }

    fn set_color_fg(mut self, fg: Color) -> Self {
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self
    }

    fn set_color_bg(mut self, bg: Color) -> Self {
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
        self
    }

    fn get_deco(&self) -> u8 { DECO_DEFAULT }

    fn set_deco(self, _deco: u8) -> Self { self }
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

    fn get_bytes(&self) -> &[u8] {
        &self.buf
    }

    fn get_color_fg(&self) -> Color { Color::from_code(self.buf[0]) }

    fn get_color_bg(&self) -> Color { Color::from_code(self.buf[1]) }

    fn set_color(mut self, fg: Color, bg: Color) -> Self {
        // Convert standard colors to RGB cube before storing their codes.
        // This avoids storing 0x00 code for standard colors.
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
        self
    }

    fn set_color_fg(mut self, fg: Color) -> Self {
        self.buf[0] = fg.standard_to_rgb_cube().get_code();
        self
    }

    fn set_color_bg(mut self, bg: Color) -> Self {
        self.buf[1] = bg.standard_to_rgb_cube().get_code();
        self
    }

    fn get_deco(&self) -> u8 { self.buf[2] }

    fn set_deco(mut self, deco: u8) -> Self {
        self.buf[2] = deco;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setters_return_updated_color_and_decorations() {
        let attr = CharAttr16ColorsDeco::new()
            .set_color_fg(Color::RED)
            .set_color_bg(Color::BLUE)
            .set_bold()
            .set_italic()
            .set_underlined()
            .set_strikethrough();

        assert!(attr.get_color_fg() == Color::RED);
        assert!(attr.get_color_bg() == Color::BLUE);
        assert!(attr.is_bold());
        assert!(attr.is_italic());
        assert!(attr.is_underlined());
        assert!(attr.is_strikethrough());

        let attr = attr
            .clear_bold()
            .clear_italic()
            .clear_underlined()
            .clear_strikethrough();

        assert!(!attr.is_bold());
        assert!(!attr.is_italic());
        assert!(!attr.is_underlined());
        assert!(!attr.is_strikethrough());
    }

    #[test]
    fn unsupported_setters_return_unchanged_attribute() {
        let attr = CharAttrMonoColor::new()
            .set_color_fg(Color::RED)
            .set_color_bg(Color::BLUE)
            .set_bold();

        assert!(attr == CharAttrMonoColor);
    }
}
