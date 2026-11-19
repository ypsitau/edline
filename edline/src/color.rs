//-----------------------------------------------------------------------------
// Color
//-----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Color {
    code: u8,
}

impl Color {
    pub const BLACK: Color = Color { code: 0 };
    pub const RED: Color = Color { code: 1 };
    pub const GREEN: Color = Color { code: 2 };
    pub const YELLOW: Color = Color { code: 3 };
    pub const BLUE: Color = Color { code: 4 };
    pub const MAGENTA: Color = Color { code: 5 };
    pub const CYAN: Color = Color { code: 6 };
    pub const WHITE: Color = Color { code: 7 };
    pub const LIGHT_BLACK: Color = Color { code: 8 };
    pub const LIGHT_RED: Color = Color { code: 9 };
    pub const LIGHT_GREEN: Color = Color { code: 10 };
    pub const LIGHT_YELLOW: Color = Color { code: 11 };
    pub const LIGHT_BLUE: Color = Color { code: 12 };
    pub const LIGHT_MAGENTA: Color = Color { code: 13 };
    pub const LIGHT_CYAN: Color = Color { code: 14 };
    pub const LIGHT_WHITE: Color = Color { code: 15 };

    pub fn from_code(code: u8) -> Self {
        Self { code }
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        // Convert RGB to ANSI 8-bit color
        let code = 16 + (36 * (r / 51)) + (6 * (g / 51)) + (b / 51);
        Self { code }
    }

    pub fn from_gray(gray: u8) -> Self {
        // Convert grayscale to ANSI 8-bit color
        let code = if gray == 0 { 16 } else if gray == 255 { 231 } else {
            232 + (gray as u16 * 24 / 256) as u8
        };
        Self { code }
    }

    pub fn standard_to_rgb_cube(self) -> Self {
        if self.code < 16 {
            let (r, g, b) = self.get_rgb();
            Self::from_rgb(r, g, b)
        } else {
            self
        }
    }

    pub fn get_code(&self) -> u8 {
        self.code
    }

    pub fn get_rgb(&self) -> (u8, u8, u8) {
        let code = self.code;
        if code < 16 {
            // Standard colors
            match code {
                0 => (0, 0, 0),
                1 => (128, 0, 0),
                2 => (0, 128, 0),
                3 => (128, 128, 0),
                4 => (0, 0, 128),
                5 => (128, 0, 128),
                6 => (0, 128, 128),
                7 => (192, 192, 192),
                8 => (128, 128, 128),
                9 => (255, 0, 0),
                10 => (0, 255, 0),
                11 => (255, 255, 0),
                12 => (0, 0, 255),
                13 => (255, 0, 255),
                14 => (0, 255, 255),
                15 => (255, 255, 255),
                _ => (0, 0, 0),
            }
        } else if code >= 232 {
            // Grayscale colors
            let gray = (code - 232) * 10 + 8;
            (gray, gray, gray)
        } else {
            // 8-bit color cube
            let code = code - 16;
            let r = (code / 36) % 6;
            let g = (code / 6) % 6;
            let b = code % 6;
            (r * 51, g * 51, b * 51)
        }
    }
}
