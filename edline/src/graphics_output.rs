use embedded_hal_async as hal_async;
use embedded_graphics as eg;
use embedded_graphics::mono_font::ascii as eg_fonts;
use embedded_graphics::prelude::*;

/// Text output with a blinking underline cursor.
///
/// Two black pixel rows below each text row are reserved for the cursor.
/// The caller must poll [`crate::Editor::wait_cursor_blink`] while waiting for
/// input; [`crate::simple_task_graphics`] does this automatically.
pub struct GraphicsOutput<DrawTarget, ColorType, DelayNs, FnRefresh> {
    target: DrawTarget,
    color_type: core::marker::PhantomData<ColorType>,
    delay: DelayNs,
    fn_refresh: FnRefresh,
    font: &'static eg::mono_font::MonoFont<'static>,
    char_buf: heapless::Vec<u8, 8>,
    pt: Point,
    x_advance: i32,
    y_advance: i32,
    cursor_visible: bool,
}

impl<DrawTarget, ColorType, DelayNs, FnRefresh> GraphicsOutput<DrawTarget, ColorType, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
{
    pub fn new(target: DrawTarget, delay: DelayNs, fn_refresh: FnRefresh) -> Self {
        let font = &eg_fonts::FONT_10X20;
        Self {
            target,
            color_type: core::marker::PhantomData,
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

impl<DrawTarget, ColorType, DelayNs, FnRefresh> GraphicsOutput<DrawTarget, ColorType, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    ColorType: eg::pixelcolor::RgbColor,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
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

impl<DrawTarget, ColorType, DelayNs, FnRefresh> crate::editor::Output
    for GraphicsOutput<DrawTarget, ColorType, DelayNs, FnRefresh>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    DelayNs: hal_async::delay::DelayNs,
    ColorType: eg::pixelcolor::RgbColor,
    FnRefresh: FnMut(&mut DrawTarget),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Editor, Output, simple_task_common};
    use eg::mock_display::MockDisplay;
    use eg::pixelcolor::Rgb888;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    fn display() -> MockDisplay<Rgb888> {
        let mut display = MockDisplay::new();
        display.set_allow_overdraw(true);
        display.set_allow_out_of_bounds_drawing(true);
        display
    }

    struct TestDelay(Rc<Cell<bool>>);

    impl hal_async::delay::DelayNs for TestDelay {
        async fn delay_ns(&mut self, ns: u32) {
            assert_eq!(ns, 500_000_000);
            core::future::poll_fn(|_| {
                if self.0.replace(false) { Poll::Ready(()) } else { Poll::Pending }
            }).await;
        }
    }

    fn run_ready(future: impl core::future::Future<Output = ()>) {
        let mut future = core::pin::pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        assert!(future.as_mut().poll(&mut cx).is_ready());
    }

    #[test]
    fn cursor_blinks_moves_and_preserves_text() {
        let tick = Rc::new(Cell::new(false));
        let mut output = GraphicsOutput::new(display(), TestDelay(tick.clone()), |_| {});
        run_ready(output.write(b"AB"));
        let text: Vec<_> = (0..20).flat_map(|y| (0..20).map(move |x| Point::new(x, y)))
            .map(|point| output.target.get_pixel(point)).collect();
        assert_eq!(output.target.get_pixel(Point::new(20, 20)), Some(Rgb888::GREEN));

        tick.set(true);
        run_ready(output.wait_cursor_blink());
        assert_eq!(output.target.get_pixel(Point::new(20, 20)), Some(Rgb888::BLACK));
        tick.set(true);
        run_ready(output.wait_cursor_blink());
        assert_eq!(output.target.get_pixel(Point::new(20, 20)), Some(Rgb888::GREEN));

        run_ready(output.move_cursor_left_n(1));
        assert_eq!(output.target.get_pixel(Point::new(20, 20)), Some(Rgb888::BLACK));
        assert_eq!(output.target.get_pixel(Point::new(10, 20)), Some(Rgb888::GREEN));
        let after: Vec<_> = (0..20).flat_map(|y| (0..20).map(move |x| Point::new(x, y)))
            .map(|point| output.target.get_pixel(point)).collect();
        assert_eq!(after, text);

        tick.set(true);
        run_ready(output.wait_cursor_blink());
        run_ready(output.move_cursor_right_n(1));
        assert_eq!(output.target.get_pixel(Point::new(10, 20)), Some(Rgb888::BLACK));
        assert_eq!(output.target.get_pixel(Point::new(20, 20)), Some(Rgb888::GREEN));
        run_ready(output.move_cursor_left_n(1));

        run_ready(output.delete_to_line_end());
        assert_eq!(output.target.get_pixel(Point::new(10, 20)), Some(Rgb888::GREEN));
        run_ready(output.new_line());
        assert_eq!(output.target.get_pixel(Point::new(10, 20)), Some(Rgb888::BLACK));
        assert_eq!(output.target.get_pixel(Point::new(0, 42)), Some(Rgb888::GREEN));
    }

    struct TestReader {
        key: Rc<Cell<Option<u8>>>,
        starts: Rc<Cell<usize>>,
    }

    impl embedded_io_async::ErrorType for TestReader {
        type Error = core::convert::Infallible;
    }

    impl embedded_io_async::Read for TestReader {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.starts.set(self.starts.get() + 1);
            core::future::poll_fn(|_| match self.key.take() {
                Some(key) => {
                    buf[0] = key;
                    Poll::Ready(Ok(1))
                }
                None => Poll::Pending,
            }).await
        }
    }

    #[test]
    fn input_wait_keeps_read_alive_across_blinks() {
        let tick = Rc::new(Cell::new(false));
        let key = Rc::new(Cell::new(None));
        let starts = Rc::new(Cell::new(0));
        let frames = Rc::new(RefCell::new(Vec::new()));
        let captured = frames.clone();
        let output = GraphicsOutput::new(display(), TestDelay(tick.clone()), move |target: &mut MockDisplay<Rgb888>| {
            captured.borrow_mut().push(target.get_pixel(Point::new(20, 20)));
        });
        let mut line = [0; 32];
        let mut history = [0; 64];
        let editor = Editor::new(&mut line, &mut history, output);
        let reader = TestReader { key: key.clone(), starts: starts.clone() };
        let mut task = core::pin::pin!(simple_task_common(editor, reader, true));
        let mut cx = Context::from_waker(Waker::noop());

        assert!(task.as_mut().poll(&mut cx).is_pending());
        assert_eq!(frames.borrow().last(), Some(&Some(Rgb888::GREEN)));
        for expected in [Rgb888::BLACK, Rgb888::GREEN] {
            tick.set(true);
            assert!(task.as_mut().poll(&mut cx).is_pending());
            assert_eq!(frames.borrow().last(), Some(&Some(expected)));
            assert_eq!(starts.get(), 1);
        }
        key.set(Some(3));
        assert!(task.as_mut().poll(&mut cx).is_ready());
        assert_eq!(starts.get(), 1);
    }
}
