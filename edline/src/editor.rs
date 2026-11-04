use core::fmt::Write as _;
use core::future::Future;
use embedded_io_async as io_async;
use heapless::String;
use static_cell::StaticCell;
use crate::round_buffer::RoundBuffer;

pub trait Output {
    type Error;

    fn write(&mut self, data: &[u8]) -> impl Future<Output = ()>;
    fn new_line(&mut self) -> impl Future<Output = ()>; 
    fn save_cursor_pos(&mut self) -> impl Future<Output = ()>;
    fn restore_cursor_pos(&mut self) -> impl Future<Output = ()>;
    fn move_cursor_left_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn move_cursor_right_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn delete_to_line_end(&mut self) -> impl Future<Output = ()>;
}

pub struct DummyOutput;

impl Output for DummyOutput {
    type Error = ();

    async fn write(&mut self, _data: &[u8]) {}
    async fn new_line(&mut self) {}
    async fn save_cursor_pos(&mut self) {}
    async fn restore_cursor_pos(&mut self) {}
    async fn move_cursor_left_n(&mut self, _n: usize) {}
    async fn move_cursor_right_n(&mut self, _n: usize) {}
    async fn delete_to_line_end(&mut self) {}
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

    async fn new_line(&mut self) {
        self.writer.write_all(b"\r\n").await.unwrap();
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
    history_buf: RoundBuffer<'a>,
    ibyte_cur: usize,
    ibyte_end: usize,
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    pub fn new(output: OutputT, line_buf: &'a mut [u8], history_buf: &'a mut [u8]) -> Self {
        Self {
            output,
            line_buf,
            history_buf: RoundBuffer::new(history_buf),
            ibyte_cur: 0,
            ibyte_end: 0,
        }
    }

    pub async fn start(&mut self) {
        self.output.save_cursor_pos().await;
    }

    pub async fn process_key(&mut self, key: serkey::Key) {
        //println!("Received key: {:?}", key);
        match key {
            serkey::Key::CookedChar(c) => { self.insert_char(c).await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::A) => { self.move_cursor_to_line_start().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::B) => { self.move_cursor_left().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::D) => { self.delete_char().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::E) => { self.move_cursor_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::F) => { self.move_cursor_right().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::H) => { self.backspace().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::J) => { self.enter().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::K) => { self.delete_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::N) => { self.recall_history_next().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::P) => { self.recall_history_prev().await; }
            serkey::Key::A(attr) if attr.is_ctrl() => { self.move_cursor_to_line_start().await; }
            serkey::Key::B(attr) if attr.is_ctrl() => { self.move_cursor_left().await; }
            serkey::Key::D(attr) if attr.is_ctrl() => { self.delete_char().await; }
            serkey::Key::E(attr) if attr.is_ctrl() => { self.move_cursor_to_line_end().await; }
            serkey::Key::F(attr) if attr.is_ctrl() => { self.move_cursor_right().await; }
            serkey::Key::H(attr) if attr.is_ctrl() => { self.backspace().await; }
            serkey::Key::J(attr) if attr.is_ctrl() => { self.enter().await; }
            serkey::Key::K(attr) if attr.is_ctrl() => { self.delete_to_line_end().await; }
            serkey::Key::N(attr) if attr.is_ctrl() => { self.recall_history_next().await; }
            serkey::Key::P(attr) if attr.is_ctrl() => { self.recall_history_prev().await; }
            serkey::Key::Enter(attr) if attr.is_none() => { self.enter().await; }
            serkey::Key::Delete(attr) if attr.is_none() => { self.delete_char().await; }
            serkey::Key::Backspace(attr) if attr.is_none() => { self.backspace().await; }
            serkey::Key::Up(attr) if attr.is_none() => { self.recall_history_prev().await; }
            serkey::Key::Down(attr) if attr.is_none() => { self.recall_history_next().await; }
            serkey::Key::Left(attr) if attr.is_none() => { self.move_cursor_left().await; }
            serkey::Key::Right(attr) if attr.is_none() => { self.move_cursor_right().await; }
            serkey::Key::Home(attr) if attr.is_none() => { self.move_cursor_to_line_start().await; }
            serkey::Key::End(attr) if attr.is_none() => { self.move_cursor_to_line_end().await; }
            _ => {}
        }
    }

    pub async fn enter(&mut self) {
        if self.ibyte_end > 0 {
            self.history_buf.push(&self.line_buf[..self.ibyte_end]).unwrap();
        }
        self.output.new_line().await;
        self.ibyte_cur = 0;
        self.ibyte_end = 0;
    }

    pub async fn recall_history_prev(&mut self) {
        if let Some((first, second, last_item)) = self.history_buf.move_cursor_backward() {
            self.line_buf[..first.len()].copy_from_slice(first);
            self.line_buf[first.len()..first.len() + second.len()].copy_from_slice(second);
            self.ibyte_end = first.len() + second.len();
            self.ibyte_cur = self.ibyte_end;
            self.output.restore_cursor_pos().await;
            self.output.write(&self.line_buf[..self.ibyte_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn recall_history_next(&mut self) {
        if let Some((first, second, last_item)) = self.history_buf.move_cursor_forward() {
            self.line_buf[..first.len()].copy_from_slice(first);
            self.line_buf[first.len()..first.len() + second.len()].copy_from_slice(second);
            self.ibyte_end = first.len() + second.len();
            self.ibyte_cur = self.ibyte_end;
            self.output.restore_cursor_pos().await;
            self.output.write(&self.line_buf[..self.ibyte_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn insert_char(&mut self, c: char) {
        let nbytes = self.insert_char_to_buf(self.ibyte_cur, c);
        if nbytes == 0 { return; }
        let ibyte_next = self.ibyte_cur + nbytes;
        if self.ibyte_end < ibyte_next {
            self.ibyte_end = ibyte_next;
        }
        self.output.write(&self.line_buf[self.ibyte_cur..self.ibyte_end]).await;
        if ibyte_next < self.ibyte_end {
            self.output.move_cursor_left_n(self.calc_width(ibyte_next, self.ibyte_end)).await;
        }
        self.ibyte_cur = ibyte_next;
    }

    pub async fn delete_char(&mut self) {
        if self.ibyte_cur >= self.ibyte_end { return; }
        self.delete_char_from_buf(self.ibyte_cur);
        self.output.write(&self.line_buf[self.ibyte_cur..self.ibyte_end]).await;
        self.output.delete_to_line_end().await;
        if self.ibyte_cur < self.ibyte_end {
            self.output.move_cursor_left_n(self.calc_width(self.ibyte_cur, self.ibyte_end)).await;
        }
    }

    pub async fn delete_to_line_end(&mut self) {
        if self.ibyte_cur >= self.ibyte_end { return; }
        self.output.delete_to_line_end().await;
        self.ibyte_end = self.ibyte_cur;
    }

    pub async fn backspace(&mut self) {
        if self.move_cursor_left().await {
            self.delete_char().await;
        }
    }

    pub async fn move_cursor_to_line_start(&mut self) {
        if self.ibyte_cur == 0 { return; }
        self.output.move_cursor_left_n(self.calc_width(0, self.ibyte_cur)).await;
        self.ibyte_cur = 0;
    }

    pub async fn move_cursor_to_line_end(&mut self) {
        if self.ibyte_cur == self.ibyte_end { return; }
        self.output.move_cursor_right_n(self.calc_width(self.ibyte_cur, self.ibyte_end)).await;
        self.ibyte_cur = self.ibyte_end;
    }

    pub async fn move_cursor_left(&mut self) -> bool {
        if self.ibyte_cur == 0 { return false; }
        let mut ibyte_next = self.ibyte_cur - 1;
        while ibyte_next > 0 && self.line_buf[ibyte_next] & 0xC0 == 0x80 {
            ibyte_next -= 1;
        }
        self.output.move_cursor_left_n(self.calc_width(ibyte_next, self.ibyte_cur)).await;
        self.ibyte_cur = ibyte_next;
        true
    }

    pub async fn move_cursor_right(&mut self) -> bool {
        if self.ibyte_cur >= self.ibyte_end { return false; }
        let mut ibyte_next = self.ibyte_cur + 1;
        while ibyte_next < self.ibyte_end && self.line_buf[ibyte_next] & 0xC0 == 0x80 {
            ibyte_next += 1;
        }
        self.output.move_cursor_right_n(self.calc_width(self.ibyte_cur, ibyte_next)).await;
        self.ibyte_cur = ibyte_next;
        true
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

    pub fn calc_width(&self, ibyte_start: usize, ibyte_end: usize) -> usize {
        let text = core::str::from_utf8(&self.line_buf[ibyte_start..ibyte_end])
            .expect("line buffer contains invalid UTF-8");
        unicode_width::UnicodeWidthStr::width(text)
    }
}

impl<'a, Writer: io_async::Write> Editor<'a, SerialOutput<Writer>> {
    pub fn with_serial_output(writer: Writer, line_buf: &'a mut [u8], history_buf: &'a mut [u8]) -> Self {
        Self::new(SerialOutput::new(writer), line_buf, history_buf)
    }
}

#[allow(dead_code)]
pub async fn simple_task(mut writer: impl io_async::Write, mut reader: impl io_async::Read, enable_break: bool) {
    writer.write_all(b"Starting editor...\r\n").await.ok();
    writer.flush().await.ok();
    let line_buf = {
        const BUFSIZE: usize = 128;
        static STATIC_CELL: StaticCell<[u8; BUFSIZE]> = StaticCell::new();
        STATIC_CELL.init([0u8; BUFSIZE])
    };
    let history_buf = {
        const BUFSIZE: usize = 1024;
        static STATIC_CELL: StaticCell<[u8; BUFSIZE]> = StaticCell::new();
        STATIC_CELL.init([0u8; BUFSIZE])
    };
    let mut editor = Editor::with_serial_output(writer, line_buf, history_buf);
    let mut serkey_reader = serkey::Reader::new();
    editor.start().await;
    loop {
        if let Ok(key) = serkey_reader.read_key(&mut reader).await {
            if enable_break {
                if let serkey::Key::CookedCtrl(serkey::Ctrl::C) = key { break; }
            }
            editor.process_key(key).await;
        }
    }
}
