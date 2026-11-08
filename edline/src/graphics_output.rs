use embedded_graphics as eg;

pub struct GraphicsOutput<DrawTarget> {
    target: DrawTarget,
}

impl<DrawTarget: eg::draw_target::DrawTarget> GraphicsOutput<DrawTarget> {
    pub fn new(target: DrawTarget) -> Self {
        Self { target }
    }
}

impl<DrawTarget> crate::editor::Output for GraphicsOutput<DrawTarget> {
    type Error = ();

    async fn write(&mut self, _data: &[u8]) {}
    async fn new_line(&mut self) {}
    async fn move_cursor_left_n(&mut self, _n: usize) {}
    async fn move_cursor_right_n(&mut self, _n: usize) {}
    async fn delete_to_line_end(&mut self) {}
}
