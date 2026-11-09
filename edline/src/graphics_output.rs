use embedded_graphics as eg;
use embedded_graphics::mono_font::ascii as eg_fonts;
use embedded_graphics::prelude::*;

pub struct GraphicsOutput<DrawTarget, ColorType, FnRefresh> {
    target: DrawTarget,
    color_type: core::marker::PhantomData<ColorType>,
    fn_refresh: FnRefresh,
    font: &'static eg::mono_font::MonoFont<'static>,
    char_buf: heapless::Vec<u8, 8>,
    pt: Point,
    x_advance: i32,
    y_advance: i32,
}

impl<DrawTarget, ColorType, FnRefresh> GraphicsOutput<DrawTarget, ColorType, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget,
    FnRefresh: FnMut(&mut DrawTarget),
{
    pub fn new(target: DrawTarget, fn_refresh: FnRefresh) -> Self {
        let font = &eg_fonts::FONT_10X20;
        Self {
            target,
            color_type: core::marker::PhantomData,
            fn_refresh,
            font,
            char_buf: heapless::Vec::new(),
            pt: Point::new(0, 0),
            x_advance: font.character_size.width as i32,
            y_advance: font.character_size.height as i32,
        }
    }
    fn clear_char_buf(&mut self) {
        self.char_buf.clear();
    }
    fn refresh(&mut self) {
        (self.fn_refresh)(&mut self.target);
    }
}

impl<DrawTarget, ColorType, FnRefresh> crate::editor::Output for GraphicsOutput<DrawTarget, ColorType, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    ColorType: eg::pixelcolor::RgbColor,
    FnRefresh: FnMut(&mut DrawTarget),
{
    type Error = ();

    async fn write(&mut self, data: &[u8]) {
        let text_style = eg::text::TextStyleBuilder::new()
            .alignment(eg::text::Alignment::Left).baseline(eg::text::Baseline::Top).build();
        let character_style = eg::mono_font::MonoTextStyleBuilder::new()
            .font(self.font).text_color(ColorType::GREEN).build();
        for x in data {
            if *x & 0xC0 == 0x80 {
                self.char_buf.push(*x).ok();
            } else {
                if !self.char_buf.is_empty() {
                    eg::text::Text::with_text_style(
                        str::from_utf8(&self.char_buf).unwrap_or(""), self.pt, character_style, text_style)
                        .draw(&mut self.target).ok();
                    self.pt.x += self.x_advance;
                    self.clear_char_buf();
                }
                if *x & 0xC0 == 0xC0 {
                    self.char_buf.push(*x).ok();
                } else {
                    eg::text::Text::with_text_style(
                        str::from_utf8(&[*x]).unwrap_or(""), self.pt, character_style, text_style)
                        .draw(&mut self.target).ok();
                    self.pt.x += self.x_advance;
                }
            }
        }
        self.refresh();
    }
    async fn new_line(&mut self) {
        self.pt.x = 0;
        self.pt.y += self.y_advance;
    }
    async fn move_cursor_left_n(&mut self, n: usize) {
        self.pt.x -= self.x_advance * n as i32;
    }
    async fn move_cursor_right_n(&mut self, n: usize) {
        self.pt.x += self.x_advance * n as i32;
    }
    async fn delete_to_line_end(&mut self) {
        
    }

}
