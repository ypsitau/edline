use core::fmt::Write as _;
use core::future::Future;
use embedded_io_async as io_async;
use heapless::String;
use static_cell::StaticCell;
use crate::round_buffer::RoundBuffer;

pub enum Event {
    None,
    Entered,
}

pub trait Output {
    type Error;

    fn write(&mut self, data: &[u8]) -> impl Future<Output = ()>;
    fn new_line(&mut self) -> impl Future<Output = ()>; 
    fn move_cursor_left_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn move_cursor_right_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn delete_to_line_end(&mut self) -> impl Future<Output = ()>;
}

pub struct DummyOutput;

impl Output for DummyOutput {
    type Error = ();

    async fn write(&mut self, _data: &[u8]) {}
    async fn new_line(&mut self) {}
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

    async fn move_cursor_left_n(&mut self, n: usize) {
        if n > 0 {
            let mut sbuf = String::<64>::new();
            write!(sbuf, "\x1b[{}D", n).unwrap();
            self.writer.write_all(sbuf.as_bytes()).await.unwrap();
            self.writer.flush().await.unwrap();
        }
    }

    async fn move_cursor_right_n(&mut self, n: usize) {
        if n > 0 {
            let mut sbuf = String::<64>::new();
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

pub struct Editor<'a, Output> {
    output: Output,
    line_buf: &'a mut [u8],
    history_buf: RoundBuffer<'a>,
    idx_cur: usize,
    idx_end: usize,
    range_result: (usize, usize),
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    pub fn new(output: OutputT, line_buf: &'a mut [u8], history_buf: &'a mut [u8]) -> Self {
        Self {
            output,
            line_buf,
            history_buf: RoundBuffer::new(history_buf),
            idx_cur: 0,
            idx_end: 0,
            range_result: (0, 0),
        }
    }

    pub async fn process_key(&mut self, key: serkey::Key) -> Event {
        //println!("Received key: {:?}", key);
        let mut event = Event::None;
        let mut history_operation = false;
        self.range_result = (0, 0);
        match key {
            serkey::Key::CookedChar(c) => { self.insert_char(c).await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::A) => { self.move_cursor_to_line_start().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::B) => { self.move_cursor_left().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::D) => { self.delete_char().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::E) => { self.move_cursor_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::F) => { self.move_cursor_right().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::H) => { self.backspace().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::J) => { self.enter().await; event = Event::Entered; }
            serkey::Key::CookedCtrl(serkey::Ctrl::K) => { self.delete_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::N) => { self.recall_history_next().await; history_operation = true; }
            serkey::Key::CookedCtrl(serkey::Ctrl::P) => { self.recall_history_prev().await; history_operation = true; }
            serkey::Key::A(attr) if attr.is_ctrl() => { self.move_cursor_to_line_start().await; }
            serkey::Key::B(attr) if attr.is_ctrl() => { self.move_cursor_left().await; }
            serkey::Key::D(attr) if attr.is_ctrl() => { self.delete_char().await; }
            serkey::Key::E(attr) if attr.is_ctrl() => { self.move_cursor_to_line_end().await; }
            serkey::Key::F(attr) if attr.is_ctrl() => { self.move_cursor_right().await; }
            serkey::Key::H(attr) if attr.is_ctrl() => { self.backspace().await; }
            serkey::Key::J(attr) if attr.is_ctrl() => { self.enter().await; event = Event::Entered; }
            serkey::Key::K(attr) if attr.is_ctrl() => { self.delete_to_line_end().await; }
            serkey::Key::N(attr) if attr.is_ctrl() => { self.recall_history_next().await; history_operation = true; }
            serkey::Key::P(attr) if attr.is_ctrl() => { self.recall_history_prev().await; history_operation = true; }
            serkey::Key::Enter(attr) if attr.is_none() => { self.enter().await; event = Event::Entered; }
            serkey::Key::Delete(attr) if attr.is_none() => { self.delete_char().await; }
            serkey::Key::Backspace(attr) if attr.is_none() => { self.backspace().await; }
            serkey::Key::Up(attr) if attr.is_none() => { self.recall_history_prev().await; history_operation = true; }
            serkey::Key::Down(attr) if attr.is_none() => { self.recall_history_next().await; history_operation = true; }
            serkey::Key::Left(attr) if attr.is_none() => { self.move_cursor_left().await; }
            serkey::Key::Right(attr) if attr.is_none() => { self.move_cursor_right().await; }
            serkey::Key::Home(attr) if attr.is_none() => { self.move_cursor_to_line_start().await; }
            serkey::Key::End(attr) if attr.is_none() => { self.move_cursor_to_line_end().await; }
            _ => {}
        }
        if !history_operation { self.history_buf.reset_cursor(); }
        event
    }

    pub fn get_result(&self) -> &[u8] {
        let (idx_start, idx_end) = self.range_result;
        let nbytes_copy = (idx_end - idx_start).min(self.line_buf.len());
        &self.line_buf[idx_start..idx_start + nbytes_copy]
    }

    pub fn read_result_str<const N: usize>(&self, result_str: &mut String::<N>) -> Result<(), heapless::CapacityError> {
        result_str.clear();
        result_str.push_str(core::str::from_utf8(&self.get_result()).unwrap())?;
        Ok(())
    }

    pub async fn print(&mut self, s: &str) {
        self.output.write(s.as_bytes()).await;
    }

    pub async fn enter(&mut self) {
        let (idx_start, idx_end) = Self::strip(&self.line_buf[..self.idx_end]);
        let line_buf_stripped = &self.line_buf[idx_start..idx_end];
        if idx_start != idx_end {
            let mut skip_push = false;
            self.history_buf.reset_cursor();
            if let Some((first, second, _)) = self.history_buf.move_cursor_backward() {
                skip_push =
                    self.idx_end == first.len() + second.len() &&
                    line_buf_stripped[..first.len()].cmp(first) == core::cmp::Ordering::Equal &&
                    line_buf_stripped[first.len()..self.idx_end].cmp(second) == core::cmp::Ordering::Equal;
            }
            if !skip_push {
                self.history_buf.push(&line_buf_stripped).unwrap();
            }
        }
        self.range_result = (idx_start, idx_end);
        self.output.new_line().await;
        self.idx_cur = 0;
        self.idx_end = 0;
    }

    fn strip(buf: &[u8]) -> (usize, usize) {
        fn is_skippable(byte: u8) -> bool {
            byte == 0x00 || byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r'
        }
        let idx_start = {
            let mut idx = 0;
            while idx < buf.len() && is_skippable(buf[idx]) { idx += 1; }
            idx
        };
        let idx_end = {
            let mut idx = buf.len();
            while idx > 0 && is_skippable(buf[idx - 1]) { idx -= 1; }
            idx
        };
        (idx_start, idx_end)
    }

    pub async fn recall_history_prev(&mut self) {
        let width = self.calc_width(0, self.idx_cur);
        if let Some((first, second, _)) = self.history_buf.move_cursor_backward() {
            self.line_buf[..first.len()].copy_from_slice(first);
            self.line_buf[first.len()..first.len() + second.len()].copy_from_slice(second);
            self.idx_end = first.len() + second.len();
            self.idx_cur = self.idx_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&self.line_buf[..self.idx_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn recall_history_next(&mut self) {
        let width = self.calc_width(0, self.idx_cur);
        if let Some((first, second, _)) = self.history_buf.move_cursor_forward() {
            self.line_buf[..first.len()].copy_from_slice(first);
            self.line_buf[first.len()..first.len() + second.len()].copy_from_slice(second);
            self.idx_end = first.len() + second.len();
            self.idx_cur = self.idx_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&self.line_buf[..self.idx_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn insert_char(&mut self, c: char) {
        let nbytes = self.insert_char_to_buf(self.idx_cur, c);
        if nbytes == 0 { return; }
        let idx_next = self.idx_cur + nbytes;
        if self.idx_end < idx_next {
            self.idx_end = idx_next;
        }
        self.output.write(&self.line_buf[self.idx_cur..self.idx_end]).await;
        if idx_next < self.idx_end {
            self.output.move_cursor_left_n(self.calc_width(idx_next, self.idx_end)).await;
        }
        self.idx_cur = idx_next;
    }

    pub async fn delete_char(&mut self) {
        if self.idx_cur >= self.idx_end { return; }
        self.delete_char_from_buf(self.idx_cur);
        self.output.write(&self.line_buf[self.idx_cur..self.idx_end]).await;
        self.output.delete_to_line_end().await;
        if self.idx_cur < self.idx_end {
            self.output.move_cursor_left_n(self.calc_width(self.idx_cur, self.idx_end)).await;
        }
    }

    pub async fn delete_to_line_end(&mut self) {
        if self.idx_cur >= self.idx_end { return; }
        self.output.delete_to_line_end().await;
        self.idx_end = self.idx_cur;
    }

    pub async fn backspace(&mut self) {
        if self.move_cursor_left().await {
            self.delete_char().await;
        }
    }

    pub async fn move_cursor_to_line_start(&mut self) {
        if self.idx_cur == 0 { return; }
        self.output.move_cursor_left_n(self.calc_width(0, self.idx_cur)).await;
        self.idx_cur = 0;
    }

    pub async fn move_cursor_to_line_end(&mut self) {
        if self.idx_cur == self.idx_end { return; }
        self.output.move_cursor_right_n(self.calc_width(self.idx_cur, self.idx_end)).await;
        self.idx_cur = self.idx_end;
    }

    pub async fn move_cursor_left(&mut self) -> bool {
        if self.idx_cur == 0 { return false; }
        let mut idx_next = self.idx_cur - 1;
        while idx_next > 0 && self.line_buf[idx_next] & 0xC0 == 0x80 {
            idx_next -= 1;
        }
        self.output.move_cursor_left_n(self.calc_width(idx_next, self.idx_cur)).await;
        self.idx_cur = idx_next;
        true
    }

    pub async fn move_cursor_right(&mut self) -> bool {
        if self.idx_cur >= self.idx_end { return false; }
        let mut idx_next = self.idx_cur + 1;
        while idx_next < self.idx_end && self.line_buf[idx_next] & 0xC0 == 0x80 {
            idx_next += 1;
        }
        self.output.move_cursor_right_n(self.calc_width(self.idx_cur, idx_next)).await;
        self.idx_cur = idx_next;
        true
    }

    pub fn insert_char_to_buf(&mut self, idx: usize, c: char) -> usize {
        let mut buf_enc = [0; 4];
        let bytes_insert = c.encode_utf8(&mut buf_enc).as_bytes();
        if self.idx_end + bytes_insert.len() > self.line_buf.len() {
            return 0;
        }
        if idx < self.idx_end {
            self.line_buf.copy_within(idx..self.idx_end, idx + bytes_insert.len());
        }
        self.line_buf[idx..idx + bytes_insert.len()].copy_from_slice(bytes_insert);
        self.idx_end += bytes_insert.len();
        bytes_insert.len()
    }

    pub fn delete_char_from_buf(&mut self, idx: usize) -> usize {
        if idx >= self.idx_end {
            return 0;
        }
        let mut nbytes_delete = 1;
        while idx + nbytes_delete < self.idx_end && self.line_buf[idx + nbytes_delete] & 0xC0 == 0x80 {
            nbytes_delete += 1;
        }
        self.line_buf.copy_within(idx + nbytes_delete..self.idx_end, idx);
        self.idx_end -= nbytes_delete;
        nbytes_delete
    }

    pub fn calc_width(&self, idx_start: usize, idx_end: usize) -> usize {
        let text = core::str::from_utf8(&self.line_buf[idx_start..idx_end])
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
    let mut result_str: heapless::String<128> = heapless::String::new();
    let mut editor = Editor::with_serial_output(writer, line_buf, history_buf);
    let mut serkey_reader = serkey::Reader::new();
    let prompt = "> ";
    editor.print(prompt).await;
    loop {
        if let Ok(key) = serkey_reader.read_key(&mut reader).await {
            if enable_break {
                if let serkey::Key::CookedCtrl(serkey::Ctrl::C) = key { break; }
            }
            match editor.process_key(key).await {
                Event::Entered => {
                    editor.read_result_str(&mut result_str).ok();
                    editor.print("Result: ").await;
                    editor.print(&result_str).await;
                    editor.print("\r\n").await;
                    editor.print(prompt).await;
                }
                Event::None => {}
            }
        }
    }
}
