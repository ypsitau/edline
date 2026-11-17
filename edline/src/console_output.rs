use heapless::String;
use core::fmt::Write as _;
use embedded_io_async as io_async;
use unicode_width::UnicodeWidthStr;
use crate::utf8_accumulator::Utf8Accumulator;

pub struct Config {
}

impl Config {
    pub fn default() -> Self {
        Self {}
    }
}

pub struct ConsoleOutput<Writer> {
    writer: Writer,
    n_cols: usize,
    n_rows: usize,
    pos: usize,
    _config: Config,
    utf8_acc: Utf8Accumulator,
}

impl<Writer: io_async::Write> ConsoleOutput<Writer> {
    pub fn new(writer: Writer, config: Config) -> Self {
        Self { writer, n_cols: 80, n_rows: 25, pos: 0, _config: config, utf8_acc: Utf8Accumulator::new() }
    }
}

impl<Writer: io_async::Write> crate::editor::Output for ConsoleOutput<Writer> {
    type Error = Writer::Error;

    async fn init(&mut self) {}
    
    async fn write(&mut self, data: &[u8]) {
        for byte in data {
            if self.utf8_acc.push(*byte) {
                let s = self.utf8_acc.get_result();
                if s == "\n" {
                    self.writer.write_all(b"\r\n").await.unwrap();
                    self.pos = 0;
                } else {
                    self.writer.write_all(s.as_bytes()).await.unwrap();
                    self.pos += UnicodeWidthStr::width(s);
                }
                self.utf8_acc.reset();
            }
        }
        self.writer.flush().await.unwrap();
        //self.writer.write_all(data).await.unwrap();
        //self.writer.flush().await.unwrap();
        //for byte in data {
        //    if self.utf8_acc.push(*byte) {
        //        let s = self.utf8_acc.get_result();
        //        self.i_col += UnicodeWidthStr::width(s);
        //    }
        //}
    }

    async fn print(&mut self, s: &str) {
        self.write(s.as_bytes()).await;
    }

    async fn new_line(&mut self, _line_buf_combined: &[u8]) {
        self.writer.write_all(b"\r\n").await.unwrap();
        self.writer.flush().await.unwrap();
        self.pos = 0;
        self.utf8_acc.reset();
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

    async fn query_text_area_size(&mut self) {
        self.writer.write_all(b"\x1b[18t").await.unwrap();
        self.writer.flush().await.unwrap();
    }

    async fn update_text_area_size(&mut self, n_cols: usize, n_rows: usize) {
        //println!("Updating text area size: {} cols, {} rows", n_cols, n_rows);
        self.n_cols = n_cols;
        self.n_rows = n_rows;
    }
}
