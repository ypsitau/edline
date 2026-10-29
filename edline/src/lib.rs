//#![no_std]
use core::fmt::Write;
use embedded_io_async as io_async;
use heapless::String;

pub trait Output {
    type Error;
    async fn write(&mut self, data: &[u8]);
    async fn save_cursor_pos(&mut self);
    async fn restore_cursor_pos(&mut self);
    async fn move_cursor_left_n(&mut self, n: usize);
    async fn move_cursor_right_n(&mut self, n: usize);
}

pub struct SerialOutput<Writer> {
    writer: Writer,
}

impl<Writer: io_async::Write> SerialOutput<Writer> {
    pub fn new(writer: Writer) -> Self {
        Self { writer }
    }
}

impl<Writer: io_async::Write> Output for SerialOutput<Writer> {
    type Error = Writer::Error;
    async fn write(&mut self, data: &[u8]) {
        self.writer.write_all(data).await.unwrap();
        self.writer.flush().await.unwrap();
    }
    async fn save_cursor_pos(&mut self) {
        self.writer.write_all(b"\x1b7").await.unwrap();
        self.writer.flush().await.unwrap();
    }
    async fn restore_cursor_pos(&mut self) {
        self.writer.write_all(b"\x1b8").await.unwrap();
        self.writer.flush().await.unwrap();
    }
    async fn move_cursor_left_n(&mut self, n: usize) {
        let mut sbuf = String::<64>::new();
        write!(sbuf, "\x1b[{}D", n).unwrap();
        self.writer.write_all(sbuf.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
    }
    async fn move_cursor_right_n(&mut self, n: usize) {
        let mut sbuf = String::<64>::new();
        write!(sbuf, "\x1b[{}C", n).unwrap();
        self.writer.write_all(sbuf.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
    }
}
pub struct Editor<'a, Output> {
    output: Output,
    line_buf: &'a mut [u8],
    history_buf: &'a mut [u8],
    icur: usize,
    icur_end: usize,
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    pub fn new(output: OutputT, line_buf: &'a mut [u8], history_buf: &'a mut [u8]) -> Self {
        Self {
            output,
            line_buf,
            history_buf,
            icur: 0,
            icur_end: 0,
        }
    }
    pub async fn start(&mut self) {
        self.output.save_cursor_pos().await;
    }
    pub async fn process_key(&mut self, key: serkey::Key) {
        //println!("Received key: {:?}", key);
        match key {
            serkey::Key::CookedChar(c) => {
                if self.icur < self.line_buf.len() {
                    self.line_buf[self.icur] = c as u8;
                    self.icur += 1;
                    if self.icur_end < self.icur {
                        self.icur_end = self.icur;
                    }
                }
                self.output.restore_cursor_pos().await;
                self.output.write(&self.line_buf[..self.icur_end]).await;
            }
            serkey::Key::Left(attr) => {
                if attr.is_none() {
                    if self.icur > 0 {
                        self.icur -= 1;
                        self.output.move_cursor_left_n(1).await;
                    }
                }
            }
            serkey::Key::Right(attr) => {
                if attr.is_none() {
                    if self.icur < self.icur_end {
                        self.icur += 1;
                        self.output.move_cursor_right_n(1).await;
                    }
                }
            }

            //serkey::Key::Backspace => { self.line_buf.pop(); },
            _ => {}
        }
    }
}
