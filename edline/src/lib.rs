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
    async fn delete_to_line_end(&mut self);
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
    
    async fn delete_to_line_end(&mut self) {
        self.writer.write_all(b"\x1b[K").await.unwrap();
        self.writer.flush().await.unwrap();
    }
}



pub struct Editor<'a, Output> {
    output: Output,
    line_buf: &'a mut [u8],
    history_buf: &'a mut [u8],
    ibyte_cur: usize,
    ibyte_end: usize,
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    pub fn new(output: OutputT, line_buf: &'a mut [u8], history_buf: &'a mut [u8]) -> Self {
        Self {
            output,
            line_buf,
            history_buf,
            ibyte_cur: 0,
            ibyte_end: 0,
        }
    }

    pub async fn start(&mut self) {
        self.output.save_cursor_pos().await;
    }

    pub fn insert_char_to_buf(&mut self, ibyte: usize, c: char) -> usize {
        let mut buf_enc = [0; 4];
        let bytes_insert = c.encode_utf8(&mut buf_enc).as_bytes();
        if self.ibyte_end + bytes_insert.len() > self.line_buf.len() {
            return 0;
        }
        if ibyte < self.ibyte_end {
            self.line_buf.copy_within(ibyte..self.ibyte_end, ibyte + bytes_insert.len());
        }
        self.line_buf[ibyte..ibyte + bytes_insert.len()].copy_from_slice(bytes_insert);
        self.ibyte_end += bytes_insert.len();
        bytes_insert.len()
    }

    pub fn delete_char_from_buf(&mut self, ibyte: usize) -> usize {
        if ibyte >= self.ibyte_end {
            return 0;
        }
        let mut nbytes_delete = 1;
        while ibyte + nbytes_delete < self.ibyte_end && self.line_buf[ibyte + nbytes_delete] & 0xC0 == 0x80 {
            nbytes_delete += 1;
        }
        self.line_buf.copy_within(ibyte + nbytes_delete..self.ibyte_end, ibyte);
        self.ibyte_end -= nbytes_delete;
        nbytes_delete
    }

    pub fn count_chars(&self, ibyte_start: usize, ibyte_end: usize) -> usize {
        let mut count = 0;
        let mut ibyte = ibyte_start;
        while ibyte < ibyte_end {
            let byte = self.line_buf[ibyte];
            if byte & 0xC0 != 0x80 {
                count += 1;
            }
            ibyte += 1;
        }
        count
    }

    pub async fn move_cursor_left(&mut self) -> usize {
        if self.ibyte_cur == 0 { return 0; }
        let mut ibyte_next = self.ibyte_cur - 1;
        while ibyte_next > 0 && self.line_buf[ibyte_next] & 0xC0 == 0x80 {
            ibyte_next -= 1;
        }
        let nbytes = self.ibyte_cur - ibyte_next;
        self.output.move_cursor_left_n(nbytes).await;
        self.ibyte_cur = ibyte_next;
        nbytes
    }

    pub async fn move_cursor_right(&mut self) -> usize {
        if self.ibyte_cur >= self.ibyte_end { return 0; }
        let mut ibyte_next = self.ibyte_cur + 1;
        while ibyte_next < self.ibyte_end && self.line_buf[ibyte_next] & 0xC0 == 0x80 {
            ibyte_next += 1;
        }
        let nbytes = ibyte_next - self.ibyte_cur;
        self.output.move_cursor_right_n(nbytes).await;
        self.ibyte_cur = ibyte_next;
        nbytes
    }

    pub async fn insert_char(&mut self, ibyte: usize, c: char) -> usize{
        let nbytes = self.insert_char_to_buf(ibyte, c);
        if nbytes == 0 { return ibyte; }
        let ibyte_next = ibyte + nbytes;
        if self.ibyte_end < ibyte_next {
            self.ibyte_end = ibyte_next;
        }
        self.output.write(&self.line_buf[ibyte..self.ibyte_end]).await;
        if ibyte_next < self.ibyte_end {
            self.output.move_cursor_left_n(self.ibyte_end - ibyte_next).await;
        }
        ibyte_next
    }

    pub async fn delete_char(&mut self, ibyte: usize) {
        if ibyte >= self.ibyte_end { return; }
        self.delete_char_from_buf(ibyte);
        self.output.write(&self.line_buf[ibyte..self.ibyte_end]).await;
        self.output.delete_to_line_end().await;
        if ibyte < self.ibyte_end {
            self.output.move_cursor_left_n(self.ibyte_end - ibyte).await;
        }
    }

    pub async fn process_key(&mut self, key: serkey::Key) {
        //println!("Received key: {:?}", key);
        match key {
            serkey::Key::CookedChar(c) => {
                self.ibyte_cur = self.insert_char(self.ibyte_cur, c).await;
            }
            serkey::Key::Delete(attr) => {
                if attr.is_none() {
                    self.delete_char(self.ibyte_cur).await;
                }
            }
            serkey::Key::Backspace(attr) => {
                if attr.is_none() {
                    if self.move_cursor_left().await > 0 {
                        self.delete_char(self.ibyte_cur).await;
                    }
                }
            }
            serkey::Key::Left(attr) => {
                if attr.is_none() {
                    self.move_cursor_left().await;
                }
            }
            serkey::Key::Right(attr) => {
                if attr.is_none() {
                    self.move_cursor_right().await;
                }
            }
            _ => {}
        }
    }
}
