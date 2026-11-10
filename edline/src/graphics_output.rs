use embedded_hal_async as hal_async;
use embedded_graphics as eg;
use embedded_graphics::mono_font::ascii as eg_fonts;
use embedded_graphics::prelude::*;

/// Text output with a blinking underline cursor.
///
/// Two black pixel rows below each text row are reserved for the cursor.
/// The caller must poll [`crate::Editor::wait_cursor_blink`] while waiting for
/// input; [`crate::simple_task_graphics`] does this automatically.
pub struct GraphicsOutput<DrawTarget, DelayNs, FnRefresh> {
    target: DrawTarget,
    delay: DelayNs,
    fn_refresh: FnRefresh,
    font: &'static eg::mono_font::MonoFont<'static>,
    char_buf: heapless::Vec<u8, 8>,
    pt: Point,
    x_advance: i32,
    y_advance: i32,
    cursor_visible: bool,
}

impl<DrawTarget, DelayNs, FnRefresh> GraphicsOutput<DrawTarget, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
{
    pub fn new(target: DrawTarget, delay: DelayNs, fn_refresh: FnRefresh) -> Self {
        let font = &eg_fonts::FONT_10X20;
        Self {
            target,
            delay,
            fn_refresh,
            font,
            char_buf: heapless::Vec::new(),
            pt: Point::new(0, 0),
            x_advance: font.character_size.width as i32,
            y_advance: font.character_size.height as i32 + 2,
            cursor_visible: false,
        }
    }
    fn clear_char_buf(&mut self) {
        self.char_buf.clear();
    }
    fn refresh(&mut self) {
        (self.fn_refresh)(&mut self.target);
    }
}

impl<DrawTarget, DelayNs, FnRefresh, ColorType> GraphicsOutput<DrawTarget, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    FnRefresh: FnMut(&mut DrawTarget),
    DelayNs: hal_async::delay::DelayNs,
    ColorType: eg::pixelcolor::RgbColor,
{
    fn draw_cursor(&mut self, visible: bool) {
        let color = if visible { ColorType::GREEN } else { ColorType::BLACK };
        let position = self.pt + Point::new(0, self.font.character_size.height as i32);
        eg::primitives::Rectangle::new(position, Size::new(self.font.character_size.width, 2))
            .into_styled(eg::primitives::PrimitiveStyle::with_fill(color))
            .draw(&mut self.target).unwrap_or_else(|_| panic!("failed to draw cursor"));
        self.cursor_visible = visible;
    }

    fn hide_cursor(&mut self) {
        if self.cursor_visible { self.draw_cursor(false); }
    }

    fn show_cursor(&mut self) {
        self.draw_cursor(true);
        self.refresh();
    }
}

impl<DrawTarget, DelayNs, FnRefresh, ColorType> crate::editor::Output
    for GraphicsOutput<DrawTarget, DelayNs, FnRefresh>
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
        self.target.clear(ColorType::BLACK).ok();
        self.refresh();
    }

    async fn write(&mut self, data: &[u8]) {
        self.hide_cursor();
        let text_style = eg::text::TextStyleBuilder::new()
            .alignment(eg::text::Alignment::Left).baseline(eg::text::Baseline::Top).build();
        let character_style = eg::mono_font::MonoTextStyleBuilder::new()
            .font(self.font).text_color(ColorType::GREEN).background_color(ColorType::BLACK).build();
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
            .into_styled(eg::primitives::PrimitiveStyle::with_fill(ColorType::BLACK))
            .draw(&mut self.target).ok();
        self.show_cursor();
    }

    async fn wait_cursor_blink(&mut self) {
        self.delay.delay_ms(500).await;
        self.draw_cursor(!self.cursor_visible);
        self.refresh();
    }
}
