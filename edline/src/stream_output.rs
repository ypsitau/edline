use heapless::String;
use core::fmt::Write as _;
use embedded_io_async as io_async;

pub struct SerialOutput<Writer> {
    writer: Writer,
}

impl<Writer: io_async::Write> SerialOutput<Writer> {
    pub fn new(writer: Writer) -> Self {
        Self { writer }
    }
}

impl<Writer: io_async::Write> crate::editor::Output for SerialOutput<Writer> {
    type Error = Writer::Error;


    async fn init(&mut self) {}
    
    async fn write(&mut self, data: &[u8]) {
        self.writer.write_all(data).await.unwrap();
        self.writer.flush().await.unwrap();
    }

    async fn new_line(&mut self) {
        self.writer.write_all(b"\r\n").await.unwrap();
        self.writer.flush().await.unwrap();
    }

    async fn move_cursor_left_n(&mut self, n: usize) {
        if n > 0 {
            let mut sbuf = String::<32>::new();
            write!(sbuf, "\x1b[{}D", n).unwrap();
            self.writer.write_all(sbuf.as_bytes()).await.unwrap();
            self.writer.flush().await.unwrap();
        }
    }

    async fn move_cursor_right_n(&mut self, n: usize) {
        if n > 0 {
            let mut sbuf = String::<32>::new();
            write!(sbuf, "\x1b[{}C", n).unwrap();
            self.writer.write_all(sbuf.as_bytes()).await.unwrap();
            self.writer.flush().await.unwrap();
        }
    }
    
    async fn delete_to_line_end(&mut self) {
        self.writer.write_all(b"\x1b[K").await.unwrap();
        self.writer.flush().await.unwrap();
    }
}
