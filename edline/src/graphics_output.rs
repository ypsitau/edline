use embedded_graphics as eg;
use embedded_graphics::mono_font::ascii as eg_fonts;
use embedded_graphics::prelude::*;

pub struct GraphicsOutput<DrawTarget, ColorType, FnRefresh> {
    target: DrawTarget,
    color_type: core::marker::PhantomData<ColorType>,
    fn_refresh: FnRefresh,
}

impl<DrawTarget, ColorType, FnRefresh> GraphicsOutput<DrawTarget, ColorType, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget,
    FnRefresh: FnMut(&mut DrawTarget),
{
    pub fn new(target: DrawTarget, fn_refresh: FnRefresh) -> Self {
        Self { target, color_type: core::marker::PhantomData, fn_refresh }
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

    async fn write(&mut self, _data: &[u8]) {
        let font = &eg_fonts::FONT_10X20;
        let character_style = eg::mono_font::MonoTextStyleBuilder::new()
            .font(font).text_color(ColorType::GREEN).build();
        eg::text::Text::new("Hello", Point::new(30, 30), character_style)
            .draw(&mut self.target).ok();
        self.refresh();
    }
    async fn new_line(&mut self) {}
    async fn move_cursor_left_n(&mut self, _n: usize) {}
    async fn move_cursor_right_n(&mut self, _n: usize) {}
    async fn delete_to_line_end(&mut self) {}

}
