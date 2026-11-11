use embedded_hal_async as hal_async;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use static_cell::StaticCell;
use crate::round_buffer::RoundBuffer;

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

pub struct GraphicsOutput<'a, DrawTarget: eg::draw_target::DrawTarget, DelayNs, FnRefresh> {
    target: DrawTarget,
    rollback_buf: RoundBuffer<'a>,
    config: Config<DrawTarget::Color>,
    delay: DelayNs,
    fn_refresh: FnRefresh,
    char_buf: heapless::Vec<u8, 8>,
    pt: Point,
    x_advance: i32,
    y_advance: i32,
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
            rollback_buf: RoundBuffer::new(rollback_buf),
            config,
            delay,
            fn_refresh,
            char_buf: heapless::Vec::new(),
            pt: Point::new(0, 0),
            x_advance, y_advance,
            n_cols, n_rows,
            cursor_visible: false,
            cursor_blinking_toggle: false,
        }
    }
    fn clear_char_buf(&mut self) {
        self.char_buf.clear();
    }
    fn refresh(&mut self) {
        (self.fn_refresh)(&mut self.target);
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
        let position = self.pt + Point::new(0, self.config.font.character_size.height as i32);
        eg::primitives::Rectangle::new(position, Size::new(self.config.font.character_size.width, 2))
            .into_styled(eg::primitives::PrimitiveStyle::with_fill(color))
            .draw(&mut self.target).unwrap_or_else(|_| panic!("failed to draw cursor"));
    }

    fn hide_cursor(&mut self) {
        if self.cursor_visible && self.cursor_blinking_toggle { self.draw_cursor(false); }
        self.cursor_visible = false;
    }
    
    fn show_cursor(&mut self) {
        self.cursor_visible = true;
        self.cursor_blinking_toggle = true;
        self.draw_cursor(true);
        self.refresh();
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
        self.clear_char_buf();
        self.pt = Point::zero();
        self.cursor_visible = false;
        self.target.clear(self.config.background_color).ok();
        self.refresh();
    }

    async fn write(&mut self, data: &[u8]) {
        self.hide_cursor();
        let text_style = eg::text::TextStyleBuilder::new()
            .alignment(eg::text::Alignment::Left).baseline(eg::text::Baseline::Top).build();
        let character_style = eg::mono_font::MonoTextStyleBuilder::new()
            .font(self.config.font).text_color(self.config.text_color)
            .background_color(self.config.background_color).build();
        for x in data {
            let x = *x;
            if x & 0xC0 == 0x80 {
                self.char_buf.push(x).ok();
                continue;
            }
            if !self.char_buf.is_empty() {
                if let Ok(s) = str::from_utf8(&self.char_buf) {
                    eg::text::Text::with_text_style(s, self.pt, character_style, text_style)
                        .draw(&mut self.target).ok();
                    self.pt.x += self.x_advance;
                }
                self.clear_char_buf();
            }
            if x == b'\n' {
                self.pt.x = 0;
                self.pt.y += self.y_advance;
            } else if x & 0xC0 == 0xC0 {
                self.char_buf.push(x).ok();
            } else if let Ok(s) = str::from_utf8(&[x]) {
                eg::text::Text::with_text_style(s, self.pt, character_style, text_style)
                    .draw(&mut self.target).ok();
                self.pt.x += self.x_advance;
            }
        }

        self.show_cursor();
    }

    async fn new_line(&mut self) {
        self.hide_cursor();
        self.pt.x = 0;
        self.pt.y += self.y_advance;
        self.show_cursor();
    }

    async fn move_cursor_left_n(&mut self, n: usize) {
        self.hide_cursor();
        self.pt.x -= self.x_advance * n as i32;
        self.show_cursor();
    }

    async fn move_cursor_right_n(&mut self, n: usize) {
        self.hide_cursor();
        self.pt.x += self.x_advance * n as i32;
        self.show_cursor();
    }

    async fn delete_to_line_end(&mut self) {
        self.hide_cursor();
        let size = Size::new(
            self.target.bounding_box().size.width.saturating_sub(self.pt.x.max(0) as u32),
            self.y_advance as u32);
        eg::primitives::Rectangle::new(self.pt, size)
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
