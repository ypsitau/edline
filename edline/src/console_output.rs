use heapless::String;
use core::fmt::Write as _;
use embedded_io_async as io_async;
use unicode_width::UnicodeWidthStr;
use crate::char_attr;
use crate::utf8_accumulator::Utf8Accumulator;

//-----------------------------------------------------------------------------
// ConsoleConfig
//-----------------------------------------------------------------------------
pub struct ConsoleConfig {
    query_text_area_size: bool,
}

impl ConsoleConfig {
    pub fn default() -> Self {
        Self { query_text_area_size: true }
    }

    pub fn query_text_area_size(self, query_text_area_size: bool) -> Self {
        Self { query_text_area_size, ..self }
    }
}

//-----------------------------------------------------------------------------
// ConsoleOutput
//-----------------------------------------------------------------------------
pub struct ConsoleOutput<Writer, CharAttr: char_attr::CharAttr> {
    writer: Writer,
    config: ConsoleConfig,
    utf8_acc: Utf8Accumulator,
    char_attr: CharAttr,
    ibyte_char_attr: usize,
    char_attr_prev: CharAttr,
    ncols: usize,
    nrows: usize,
    pos: usize,
    char_attr_changed: bool,
}

impl<Writer: io_async::Write, CharAttr: char_attr::CharAttr> ConsoleOutput<Writer, CharAttr> {
    pub fn new(writer: Writer, config: ConsoleConfig) -> Self {
        Self {
            writer,
            config,
            utf8_acc: Utf8Accumulator::new(),
            char_attr: CharAttr::new(),
            ibyte_char_attr: 0,
            char_attr_prev: CharAttr::new(),
            ncols: 80,
            nrows: 25,
            pos: 0,
            char_attr_changed: false,
        }
    }
}

impl<Writer: io_async::Write, CharAttr: char_attr::CharAttr> crate::editor::Output for ConsoleOutput<Writer, CharAttr> {
    type Error = Writer::Error;

    async fn init(&mut self) {
        self.utf8_acc.reset();
        self.char_attr = CharAttr::new();
        self.ibyte_char_attr = 0;
        self.char_attr_prev = CharAttr::new();
        self.pos = 0;
        self.char_attr_changed = false;
    }
    
    async fn draw(&mut self, data: &[u8]) {
        let mut sbuf = String::<32>::new();
        for byte in data {
            if self.ibyte_char_attr < CharAttr::NBYTES {
                self.char_attr.put_byte(self.ibyte_char_attr, *byte);
                self.ibyte_char_attr += 1;
            } else if self.utf8_acc.push(*byte) {
                if self.char_attr.is_changed_color_fg(&self.char_attr_prev) {
                    sbuf.clear();
                    write!(sbuf, "\x1b[38;5;{}m", self.char_attr.get_color_fg().get_code()).unwrap();
                    self.writer.write_all(sbuf.as_bytes()).await.unwrap();
                    self.char_attr_changed = true;
                }
                if self.char_attr.is_changed_color_bg(&self.char_attr_prev) {
                    sbuf.clear();
                    write!(sbuf, "\x1b[48;5;{}m", self.char_attr.get_color_bg().get_code()).unwrap();
                    self.writer.write_all(sbuf.as_bytes()).await.unwrap();
                    self.char_attr_changed = true;
                }
                if self.char_attr.is_changed_bold(&self.char_attr_prev) {
                    let s = if self.char_attr.is_bold() { "\x1b[1m" } else { "\x1b[22m" };
                    self.writer.write_all(s.as_bytes()).await.unwrap();
                    self.char_attr_changed = true;
                }
                if self.char_attr.is_changed_italic(&self.char_attr_prev) {
                    let s = if self.char_attr.is_italic() { "\x1b[3m" } else { "\x1b[23m" };
                    self.writer.write_all(s.as_bytes()).await.unwrap();
                    self.char_attr_changed = true;
                }
                if self.char_attr.is_changed_underlined(&self.char_attr_prev) {
                    let s = if self.char_attr.is_underlined() { "\x1b[4m" } else { "\x1b[24m" };
                    self.writer.write_all(s.as_bytes()).await.unwrap();
                    self.char_attr_changed = true;
                }
                if self.char_attr.is_changed_strikethrough(&self.char_attr_prev) {
                    let s = if self.char_attr.is_strikethrough() { "\x1b[7m" } else { "\x1b[27m" };
                    self.writer.write_all(s.as_bytes()).await.unwrap();
                    self.char_attr_changed = true;
                }
                let s = self.utf8_acc.get_result();
                if s == "\n" {
                    if self.char_attr_changed {
                        self.writer.write_all(b"\x1b[m").await.unwrap();
                        self.char_attr_changed = false;
                    }
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
        let char_attr = CharAttr::new();
        for byte in s.as_bytes() {
            if utf8_acc.push(*byte) {
                let s = utf8_acc.get_result();
                self.draw(char_attr.get_bytes()).await;
                self.draw(s.as_bytes()).await;
                utf8_acc.reset();
            }
        }
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
        if self.config.query_text_area_size {
            self.writer.write_all(b"\x1b[18t").await.unwrap();
            self.writer.flush().await.unwrap();
        }
    }

    async fn update_text_area_size(&mut self, n_cols: usize, n_rows: usize) {
        //println!("Updating text area size: {} cols, {} rows", n_cols, n_rows);
        self.ncols = n_cols;
        self.nrows = n_rows;
    }
}
