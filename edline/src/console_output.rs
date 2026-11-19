use heapless::String;
use core::fmt::Write as _;
use embedded_io_async as io_async;
use unicode_width::UnicodeWidthStr;
use crate::char_attr;
use crate::utf8_accumulator::Utf8Accumulator;

pub struct Config {
}

impl Config {
    pub fn default() -> Self {
        Self {}
    }
}

pub struct ConsoleOutput<Writer, CharAttr: char_attr::CharAttr> {
    writer: Writer,
    _config: Config,
    utf8_acc: Utf8Accumulator,
    char_attr: CharAttr,
    ibyte_char_attr: usize,
    char_attr_prev: CharAttr,
    n_cols: usize,
    n_rows: usize,
    pos: usize,
}

impl<Writer: io_async::Write, CharAttr: char_attr::CharAttr> ConsoleOutput<Writer, CharAttr> {
    pub fn new(writer: Writer, config: Config) -> Self {
        Self {
            writer,
            _config: config,
            utf8_acc: Utf8Accumulator::new(),
            char_attr: CharAttr::new(),
            ibyte_char_attr: 0,
            char_attr_prev: CharAttr::new(),
            n_cols: 80,
            n_rows: 25,
            pos: 0
        }
    }
}

impl<Writer: io_async::Write, CharAttr: char_attr::CharAttr> crate::editor::Output for ConsoleOutput<Writer, CharAttr> {
    type Error = Writer::Error;

    async fn init(&mut self) {}
    
    async fn write(&mut self, data: &[u8]) {
        for byte in data {
            if self.ibyte_char_attr < CharAttr::NBYTES {
                self.char_attr.put_byte(self.ibyte_char_attr, *byte);
                self.ibyte_char_attr += 1;
            } else if self.utf8_acc.push(*byte) {
                if self.char_attr != self.char_attr_prev {
                    // Apply attribute changes here if needed
                }
                let s = self.utf8_acc.get_result();
                if s == "\n" {
                    self.writer.write_all(b"\r\n").await.unwrap();
                    self.pos = 0;
                } else {
                    self.writer.write_all(s.as_bytes()).await.unwrap();
                    self.pos += UnicodeWidthStr::width(s);
                }
                self.utf8_acc.reset();
                self.char_attr_prev.clone_from(&self.char_attr);
                self.ibyte_char_attr = 0;
            }
        }
        self.writer.flush().await.unwrap();
    }

    async fn print(&mut self, s: &str) {
        let mut utf8_acc = Utf8Accumulator::new();
        let char_attr = CharAttr::default();
        for byte in s.as_bytes() {
            if utf8_acc.push(*byte) {
                let s = utf8_acc.get_result();
                self.write(char_attr.get_bytes()).await;
                self.write(s.as_bytes()).await;
                utf8_acc.reset();
            }
        }
        //self.write(s.as_bytes()).await;
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
