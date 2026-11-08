pub struct GraphicsOutput {
}

impl crate::editor::Output for GraphicsOutput {
    type Error = ();

    async fn write(&mut self, _data: &[u8]) {}
    async fn new_line(&mut self) {}
    async fn move_cursor_left_n(&mut self, _n: usize) {}
    async fn move_cursor_right_n(&mut self, _n: usize) {}
    async fn delete_to_line_end(&mut self) {}
}
