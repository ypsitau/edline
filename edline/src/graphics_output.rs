use embedded_hal_async as hal_async;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use unicode_width::UnicodeWidthStr;
use crate::editor::BaseConfig;
use crate::char_attr;
use crate::round_buffer::{WrappedRoundBuffer, RoundBufferMarker};
use crate::utf8_accumulator::Utf8Accumulator;

//-----------------------------------------------------------------------------
// GraphicsConfig
//-----------------------------------------------------------------------------
pub struct GraphicsConfig<ColorType> {
    pub base_config: BaseConfig,
    pub cursor_blinking_interval: u32,
    pub font: &'static eg::mono_font::MonoFont<'static>,
    pub text_color: ColorType,
    pub background_color: ColorType,
    pub ht_cursor: u32,
}

impl<ColorType: From<eg::pixelcolor::Rgb888>> GraphicsConfig<ColorType> {
    pub fn default() -> Self {
        Self {
            base_config: BaseConfig::default(),
            cursor_blinking_interval: 500,
            font: &eg::mono_font::ascii::FONT_6X10,
            text_color: ColorType::from(eg::pixelcolor::Rgb888::WHITE),
            background_color: ColorType::from(eg::pixelcolor::Rgb888::BLACK),
            ht_cursor: 2,
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

    pub fn ht_cursor(mut self, ht_cursor: u32) -> Self {
        self.ht_cursor = ht_cursor;
        self
    }
}

//-----------------------------------------------------------------------------
// GraphicsOutput
//-----------------------------------------------------------------------------
pub struct GraphicsOutput<'a, DrawTarget: eg::draw_target::DrawTarget, DelayNs, FnRefresh, CharAttr> {
    target: DrawTarget,
    character_style: eg::mono_font::MonoTextStyle<'static, DrawTarget::Color>,
    text_style: eg::text::TextStyle,
    rollback_buf: WrappedRoundBuffer<'a>,
    config: GraphicsConfig<DrawTarget::Color>,
    delay: DelayNs,
    fn_refresh: FnRefresh,
    utf8_acc: Utf8Accumulator,
    char_attr: CharAttr,
    ibyte_char_attr: usize,
    char_attr_prev: CharAttr,
    x_advance: i32,
    y_advance: i32,
    icol: i32,
    irow: i32,
    ncols: i32,
    nrows: i32,
    cursor_visible: bool,
    cursor_blinking_toggle: bool,
}

impl<'a, DrawTarget, DelayNs, FnRefresh, CharAttr> GraphicsOutput<'a, DrawTarget, DelayNs, FnRefresh, CharAttr>
where
    DrawTarget: eg::draw_target::DrawTarget,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
    CharAttr: char_attr::CharAttr,
{
    pub fn new(
        target: DrawTarget,
        rollback_buf: &'a mut [u8],
        config: GraphicsConfig<<DrawTarget as eg::draw_target::DrawTarget>::Color>,
        delay: DelayNs,
        fn_refresh: FnRefresh,
    ) -> Self {
        let font = config.font;
        let x_advance = font.character_size.width as i32;
        let y_advance = font.character_size.height as i32 + config.ht_cursor as i32;
        let size = target.bounding_box().size;
        let ncols = size.width as i32 / x_advance;
        let nrows = size.height as i32 / y_advance;
        Self {
            target,
            character_style: eg::mono_font::MonoTextStyleBuilder::new()
                .font(config.font).text_color(config.text_color)
                .background_color(config.background_color).build(),
            text_style: eg::text::TextStyleBuilder::new()
                .alignment(eg::text::Alignment::Left).baseline(eg::text::Baseline::Top).build(),
            rollback_buf: WrappedRoundBuffer::new(rollback_buf, ncols as usize),
            config,
            delay,
            fn_refresh,
            utf8_acc: Utf8Accumulator::new(),
            char_attr: CharAttr::new(),
            ibyte_char_attr: 0,
            char_attr_prev: CharAttr::new(),
            icol: 0,
            irow: 0,
            x_advance, y_advance,
            ncols, nrows,
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

    fn draw_rollback(&mut self, n_rows_bottom: i32) {
        let bbox_size = self.target.bounding_box().size;
        let (width, height) = (bbox_size.width as i32, bbox_size.height as i32);
        let mut utf8_acc = Utf8Accumulator::new();
        let mut char_attr = CharAttr::new();
        let mut char_attr_prev = CharAttr::new();
        let mut ibyte_char_attr = 0;
        let mut i_row = core::cmp::min(self.nrows - n_rows_bottom, self.rollback_buf.count() as i32);
        self.irow = i_row;
        let mut marker = RoundBufferMarker::new();
        while i_row > 0 && self.rollback_buf.seek_item_prev(&mut marker) {
            i_row -= 1;
            let mut i_col = 0;
            utf8_acc.reset();
            let (first, second) = self.rollback_buf.get_item(marker).unwrap();
            //println!("i_row: {} {}{}", i_row,
            //    str::from_utf8(first).unwrap(), str::from_utf8(second).unwrap());
            let y = self.calc_y(i_row);
            for byte in first.iter().chain(second.iter()) {
                if ibyte_char_attr < CharAttr::NBYTES {
                    char_attr.put_byte(ibyte_char_attr, *byte);
                    ibyte_char_attr += 1;
                } else if utf8_acc.push(*byte) {
                    if char_attr != char_attr_prev {
                        // Apply attribute changes here if needed
                    }
                    let s = utf8_acc.get_result();
                    let x = self.calc_x(i_col);
                    eg::text::Text::with_text_style(s, Point::new(x, y), self.character_style, self.text_style)
                        .draw(&mut self.target).ok();
                    i_col += UnicodeWidthStr::width(s) as i32;
                    utf8_acc.reset();
                    char_attr_prev.clone_from(&char_attr);
                    ibyte_char_attr = 0;
                }
            }
            if i_col < self.ncols {
                let pt = self.calc_point(i_col, i_row);
                eg::primitives::Rectangle::new(pt, Size::new((width - pt.x) as u32, self.y_advance as u32))
                    .into_styled(eg::primitives::PrimitiveStyle::with_fill(self.config.background_color))
                    .draw(&mut self.target).ok();
            }
        }
        if n_rows_bottom > 0 {
            let y = self.calc_y(self.nrows - n_rows_bottom);
            eg::primitives::Rectangle::new(Point::new(0, y), Size::new(width as u32, (height - y) as u32))
                .into_styled(eg::primitives::PrimitiveStyle::with_fill(self.config.background_color))
                .draw(&mut self.target).ok();
        }
    }
}

impl<DrawTarget, DelayNs, FnRefresh, CharAttr> GraphicsOutput<'_, DrawTarget, DelayNs, FnRefresh, CharAttr>
where
    DrawTarget: eg::draw_target::DrawTarget,
    FnRefresh: FnMut(&mut DrawTarget),
    DelayNs: hal_async::delay::DelayNs,
    CharAttr: char_attr::CharAttr,
{
    fn draw_cursor(&mut self, visible: bool) {
        let color = if visible { self.config.text_color } else { self.config.background_color };
        let x = self.calc_x(self.icol);
        let y = self.calc_y(self.irow) + self.config.font.character_size.height as i32;
        eg::primitives::Rectangle::new(Point::new(x, y), Size::new(self.config.font.character_size.width, self.config.ht_cursor))
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

impl<DrawTarget, DelayNs, FnRefresh, CharAttr> crate::editor::Output
    for GraphicsOutput<'_, DrawTarget, DelayNs, FnRefresh, CharAttr>
where
    DrawTarget: eg::draw_target::DrawTarget,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
    CharAttr: char_attr::CharAttr,
{
    type Error = ();
    type CharAttr = CharAttr;

    fn base_config(&self) -> &BaseConfig { &self.config.base_config }

    async fn init(&mut self) {
        self.utf8_acc.reset();
        self.ibyte_char_attr = 0;
        self.icol = 0;
        self.irow = 0;
        self.cursor_visible = false;
        self.target.clear(self.config.background_color).ok();
        self.refresh();
    }

    async fn draw(&mut self, data: &[u8]) {
        self.hide_cursor();
        for byte in data {
            if self.ibyte_char_attr < CharAttr::NBYTES {
                self.char_attr.put_byte(self.ibyte_char_attr, *byte);
                self.ibyte_char_attr += 1;
            } else if self.utf8_acc.push(*byte) {
                if self.char_attr != self.char_attr_prev {
                    // Apply attribute changes here if needed
                }
                let s = self.utf8_acc.get_result();
                if s == "\n" {
                    self.icol = 0;
                    self.irow += 1;
                } else {
                    let i_col_next = self.icol + UnicodeWidthStr::width(s) as i32;
                    if i_col_next >= self.ncols {
                        self.icol = 0;
                        self.irow += 1;
                    }
                    let pt = self.calc_point(self.icol, self.irow);
                    eg::text::Text::with_text_style(s, pt, self.character_style, self.text_style)
                        .draw(&mut self.target).ok();
                    self.icol = i_col_next;
                }
                self.utf8_acc.reset();
                self.char_attr_prev.clone_from(&self.char_attr);
                self.ibyte_char_attr = 0;
            }
        }
        self.show_cursor();
    }

    async fn print(&mut self, s: &str) {
        let mut utf8_acc = Utf8Accumulator::new();
        let char_attr = CharAttr::new();
        for byte in s.as_bytes() {
            if utf8_acc.push(*byte) {
                let s = utf8_acc.get_result();
                self.rollback_buf.push(char_attr.get_bytes()).unwrap();
                self.rollback_buf.push(s.as_bytes()).unwrap();
                utf8_acc.reset();
            }
        }
        self.hide_cursor();
        self.draw_rollback(1);
        self.show_cursor();
    }

    async fn new_line(&mut self, line_buf_combined: &[u8]) {
        self.rollback_buf.push(line_buf_combined).unwrap();
        self.rollback_buf.flush();
        self.hide_cursor();
        self.icol = 0;
        self.draw_rollback(1);
        self.show_cursor();
    }

    async fn move_cursor_left_n(&mut self, n: usize) {
        self.hide_cursor();
        self.icol -= n as i32;
        self.show_cursor();
    }

    async fn move_cursor_right_n(&mut self, n: usize) {
        self.hide_cursor();
        self.icol += n as i32;
        self.show_cursor();
    }

    async fn delete_to_line_end(&mut self) {
        let pt = self.calc_point(self.icol, self.irow);
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

    async fn query_text_area_size(&mut self) {
        // nothing to do
    }

    async fn update_text_area_size(&mut self, _n_cols: usize, _n_rows: usize) {
        // Text area size depends only on the device.      
    }
}
