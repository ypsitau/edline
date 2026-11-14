use embedded_hal_async as hal_async;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use crate::round_buffer::RoundBuffer;
use unicode_width::UnicodeWidthStr;

//-----------------------------------------------------------------------------
// Config
//-----------------------------------------------------------------------------
pub struct Config<ColorType = eg::pixelcolor::Rgb565> {
    pub cursor_blinking_interval: u32,
    pub font: &'static eg::mono_font::MonoFont<'static>,
    pub text_color: ColorType,
    pub background_color: ColorType,
}

impl<ColorType: eg::pixelcolor::RgbColor> Config<ColorType> {
    pub fn default() -> Self {
        Self {
            cursor_blinking_interval: 500,
            font: &eg::mono_font::ascii::FONT_6X10,
            text_color: ColorType::WHITE,
            background_color: ColorType::BLACK,
        }
    }
    pub fn cursor_blinking_interval(mut self, cursor_blinking_interval: u32) -> Self {
        self.cursor_blinking_interval = cursor_blinking_interval;
        self
    }
    pub fn font(mut self, font: &'static eg::mono_font::MonoFont<'static>) -> Self {
        self.font = font;
        self
    }
    pub fn text_color(mut self, text_color: ColorType) -> Self {
        self.text_color = text_color;
        self
    }
    pub fn background_color(mut self, background_color: ColorType) -> Self {
        self.background_color = background_color;
        self
    }
}

//-----------------------------------------------------------------------------
// GraphicsOutput
//-----------------------------------------------------------------------------
pub struct GraphicsOutput<'a, DrawTarget: eg::draw_target::DrawTarget, DelayNs, FnRefresh> {
    target: DrawTarget,
    character_style: eg::mono_font::MonoTextStyle<'static, DrawTarget::Color>,
    text_style: eg::text::TextStyle,
    rollback_buf: RoundBuffer<'a>,
    work_buf: &'a mut [u8],
    idx_work_buf: usize,
    config: Config<DrawTarget::Color>,
    delay: DelayNs,
    fn_refresh: FnRefresh,
    utf8_acc: Utf8Accumulator,
    x_advance: i32,
    y_advance: i32,
    i_col: i32,
    i_row: i32,
    n_cols: i32,
    n_rows: i32,
    cursor_visible: bool,
    cursor_blinking_toggle: bool,
}

impl<'a, DrawTarget, DelayNs, FnRefresh> GraphicsOutput<'a, DrawTarget, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
{
    pub fn new(
        target: DrawTarget,
        rollback_buf: &'a mut [u8],
        work_buf: &'a mut [u8],
        config: Config<<DrawTarget as eg::draw_target::DrawTarget>::Color>,
        delay: DelayNs,
        fn_refresh: FnRefresh,
    ) -> Self {
        let font = config.font;
        let x_advance = font.character_size.width as i32;
        let y_advance = font.character_size.height as i32 + 2;
        let size = target.bounding_box().size;
        let n_cols = size.width as i32 / x_advance;
        let n_rows = size.height as i32 / y_advance;
        Self {
            target,
            character_style: eg::mono_font::MonoTextStyleBuilder::new()
                .font(config.font).text_color(config.text_color)
                .background_color(config.background_color).build(),
            text_style: eg::text::TextStyleBuilder::new()
                .alignment(eg::text::Alignment::Left).baseline(eg::text::Baseline::Top).build(),
            rollback_buf: RoundBuffer::new(rollback_buf),
            work_buf,
            idx_work_buf: 0,
            config,
            delay,
            fn_refresh,
            utf8_acc: Utf8Accumulator::new(),
            i_col: 0,
            i_row: 0,
            x_advance, y_advance,
            n_cols: n_cols, n_rows,
            cursor_visible: false,
            cursor_blinking_toggle: false,
        }
    }

    fn refresh(&mut self) {
        (self.fn_refresh)(&mut self.target);
    }

    fn calc_x(&self, i_col: i32) -> i32 { i_col * self.x_advance }

    fn calc_y(&self, i_row: i32) -> i32 { i_row * self.y_advance }

    fn calc_point(&self, i_col: i32, i_row: i32) -> Point {
        Point::new(self.calc_x(i_col), self.calc_y(i_row))
    }

    fn push_rollback(rollback_buf: &mut RoundBuffer, buf: &[u8]) {
        rollback_buf.push(buf).unwrap();
    }

    fn advance_row(&mut self) {
        if self.i_row + 1 < self.n_rows {
            self.i_row += 1;
            return;
        }
        let width = self.target.bounding_box().size.width as i32;
        let mut idx_item = RoundBuffer::IDX_NONE;
        let mut utf8_acc = Utf8Accumulator::new();
        let mut i_row = self.n_rows - 1;
        let pt = self.calc_point(0, i_row);
        eg::primitives::Rectangle::new(pt, Size::new((width - pt.x) as u32, self.y_advance as u32))
            .into_styled(eg::primitives::PrimitiveStyle::with_fill(self.config.background_color))
            .draw(&mut self.target).ok();

        i_row -= 1;
        while i_row >= 0 && self.rollback_buf.seek_item_backward(&mut idx_item) {
            let mut i_col = 0;
            utf8_acc.reset();
            let (first, second) = self.rollback_buf.get_item(idx_item).unwrap();
            //println!("i_row: {} {}{}", i_row,
            //    str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
            for byte in first.iter().chain(second.iter()) {
                if utf8_acc.push(*byte) {
                    let s = utf8_acc.get_result();
                    let pt = self.calc_point(i_col, i_row);
                    eg::text::Text::with_text_style(s, pt, self.character_style, self.text_style)
                        .draw(&mut self.target).ok();
                    i_col += UnicodeWidthStr::width(s) as i32;
                    utf8_acc.reset();
                }
            }
            if i_col < self.n_cols {
                let pt = self.calc_point(i_col, i_row);
                eg::primitives::Rectangle::new(pt, Size::new((width - pt.x) as u32, self.y_advance as u32))
                    .into_styled(eg::primitives::PrimitiveStyle::with_fill(self.config.background_color))
                    .draw(&mut self.target).ok();
            }
            i_row -= 1;
        }
    }
}

impl<DrawTarget, DelayNs, FnRefresh, ColorType> GraphicsOutput<'_, DrawTarget, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    FnRefresh: FnMut(&mut DrawTarget),
    DelayNs: hal_async::delay::DelayNs,
    ColorType: eg::pixelcolor::RgbColor,
{
    fn draw_cursor(&mut self, visible: bool) {
        let color = if visible { self.config.text_color } else { self.config.background_color };
        let position = self.calc_point(self.i_col, self.i_row) + Point::new(0, self.config.font.character_size.height as i32);
        eg::primitives::Rectangle::new(position, Size::new(self.config.font.character_size.width, 2))
            .into_styled(eg::primitives::PrimitiveStyle::with_fill(color))
            .draw(&mut self.target).unwrap_or_else(|_| panic!("failed to draw cursor"));
    }

    fn show_cursor(&mut self) {
        self.draw_cursor(true);
        self.refresh();
        self.cursor_visible = true;
        self.cursor_blinking_toggle = true;
    }

    fn hide_cursor(&mut self) {
        if self.cursor_visible && self.cursor_blinking_toggle {
            self.draw_cursor(false);
            self.refresh();
        }
        self.cursor_visible = false;
    }
}

impl<DrawTarget, DelayNs, FnRefresh, ColorType> crate::editor::Output
    for GraphicsOutput<'_, DrawTarget, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
    ColorType: eg::pixelcolor::RgbColor,
{
    type Error = ();

    async fn init(&mut self) {
        self.utf8_acc.reset();
        self.i_col = 0;
        self.i_row = 0;
        self.cursor_visible = false;
        self.target.clear(self.config.background_color).ok();
        self.refresh();
    }

    async fn write(&mut self, data: &[u8]) {
        self.hide_cursor();
        for byte in data {
            if !self.utf8_acc.push(*byte) { continue; }
            let s = self.utf8_acc.get_result();
            if s == "\r" {
                // nothing to do
            } else if s == "\n" {
                self.i_col = 0;
                self.i_row += 1;
            } else {
                let i_col_next = self.i_col + UnicodeWidthStr::width(s) as i32;
                if i_col_next >= self.n_cols {
                    self.i_col = 0;
                    self.i_row += 1;
                }
                let pt = self.calc_point(self.i_col, self.i_row);
                eg::text::Text::with_text_style(s, pt, self.character_style, self.text_style)
                    .draw(&mut self.target).ok();
                self.i_col = i_col_next;
            }
            self.utf8_acc.reset();
        }
        self.show_cursor();
    }

    async fn print(&mut self, s: &str) {
        self.write(s.as_bytes()).await;
        Self::push_rollback(&mut self.rollback_buf, s.as_bytes());
    }

    async fn new_line(&mut self, prompt: &str, line_buf: &[u8]) {
        let nbytes_combined = prompt.as_bytes().len() + line_buf.len();
        let idx_next = prompt.as_bytes().len();
        self.work_buf[..idx_next].copy_from_slice(prompt.as_bytes());
        self.work_buf[idx_next..idx_next + line_buf.len()].copy_from_slice(line_buf);
        Self::push_rollback(&mut self.rollback_buf, &self.work_buf[..nbytes_combined]);
        self.hide_cursor();
        self.i_col = 0;
        self.advance_row();
        self.show_cursor();
    }

    async fn move_cursor_left_n(&mut self, n: usize) {
        self.hide_cursor();
        self.i_col -= n as i32;
        self.show_cursor();
    }

    async fn move_cursor_right_n(&mut self, n: usize) {
        self.hide_cursor();
        self.i_col += n as i32;
        self.show_cursor();
    }

    async fn delete_to_line_end(&mut self) {
        let pt = self.calc_point(self.i_col, self.i_row);
        let width = self.target.bounding_box().size.width as i32;
        if pt.x >= width { return; }
        self.hide_cursor();
        eg::primitives::Rectangle::new(pt, Size::new((width - pt.x) as u32, self.y_advance as u32))
            .into_styled(eg::primitives::PrimitiveStyle::with_fill(self.config.background_color))
            .draw(&mut self.target).ok();
        self.show_cursor();
    }

    async fn blink_cursor(&mut self) {
        self.delay.delay_ms(self.config.cursor_blinking_interval).await;
        if self.cursor_visible {
            self.cursor_blinking_toggle = !self.cursor_blinking_toggle;
            self.draw_cursor(self.cursor_blinking_toggle);
            self.refresh();
        }
    }
}

//-----------------------------------------------------------------------------
// Utf8Accumulator
//-----------------------------------------------------------------------------
struct Utf8Accumulator {
    buf: [u8; 8],
    idx: u8,
    idx_end: u8,
}

impl Utf8Accumulator {
    fn new() -> Self {
        Self { buf: [0; 8], idx: 0, idx_end: 0 }
    }

    fn reset(&mut self) {
        self.idx = 0;
        self.idx_end = 0;
    }

    fn push(&mut self, byte: u8) -> bool {
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

    fn get_result(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.idx_end as usize]) }
    }
}
