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

    pub fn insert_char(&mut self, ibyte: usize, c: char) -> usize {
        let mut buf_enc = [0; 4];
        let bytes_to_insert = c.encode_utf8(&mut buf_enc).as_bytes();
        if self.ibyte_end + bytes_to_insert.len() > self.line_buf.len() {
            return 0;
        }
        if ibyte < self.ibyte_end {
            self.line_buf.copy_within(ibyte..self.ibyte_end, ibyte + bytes_to_insert.len());
        }
        self.line_buf[ibyte..ibyte + bytes_to_insert.len()].copy_from_slice(bytes_to_insert);
        bytes_to_insert.len()
    }

    pub fn delete_char(&mut self, ibyte: usize) -> usize {
        if ibyte >= self.ibyte_end {
            return 0;
        }
        let mut nbytes = 1;
        while ibyte + nbytes < self.ibyte_end && self.line_buf[ibyte + nbytes] & 0xC0 == 0x80 {
            nbytes += 1;
        }
        self.line_buf.copy_within(ibyte + nbytes..self.ibyte_end, ibyte);
        self.ibyte_end -= nbytes;
        nbytes
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

    pub async fn process_key(&mut self, key: serkey::Key) {
        //println!("Received key: {:?}", key);
        match key {
            serkey::Key::CookedChar(c) => {
                let nbytes = self.insert_char(self.ibyte_cur, c);
                if nbytes > 0 {
                    let nchars_trail = self.count_chars(self.ibyte_cur, self.ibyte_end);
                    let ibyte_next = self.ibyte_cur + nbytes;
                    if self.ibyte_end < ibyte_next {
                        self.ibyte_end = ibyte_next;
                    }
                    self.output.write(&self.line_buf[self.ibyte_cur..self.ibyte_end]).await;
                    if nchars_trail > 1 {
                        self.output.move_cursor_left_n(nchars_trail - 1).await;
                    }
                    self.ibyte_cur = ibyte_next;
                }
            }
            serkey::Key::Delete(attr) => {
                if attr.is_none() {
                    if self.ibyte_cur < self.ibyte_end {
                        self.delete_char(self.ibyte_cur);
                        self.output.write(&self.line_buf[self.ibyte_cur..self.ibyte_end]).await;
                        let nchars_trail = self.count_chars(self.ibyte_cur, self.ibyte_end);
                        if nchars_trail > 0 {
                            self.output.move_cursor_left_n(nchars_trail).await;
                        }
                    }
                }
            }
            serkey::Key::Left(attr) => {
                if attr.is_none() {
                    if self.ibyte_cur > 0 {
                        let mut ibyte_next = self.ibyte_cur - 1;
                        while ibyte_next > 0 && self.line_buf[ibyte_next] & 0xC0 == 0x80 {
                            ibyte_next -= 1;
                        }
                        self.output.move_cursor_left_n(1).await;
                        self.ibyte_cur = ibyte_next;
                    }
                }
            }
            serkey::Key::Right(attr) => {
                if attr.is_none() {
                    if self.ibyte_cur < self.ibyte_end {
                        let mut ibyte_next = self.ibyte_cur + 1;
                        while ibyte_next < self.ibyte_end && self.line_buf[ibyte_next] & 0xC0 == 0x80 {
                            ibyte_next += 1;
                        }
                        self.output.move_cursor_right_n(1).await;
                        self.ibyte_cur = ibyte_next;
                    }
                }
            }

            //serkey::Key::Backspace => { self.line_buf.pop(); },
            _ => {}
        }
    }
}
