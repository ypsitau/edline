use core::fmt::Write as _;
use core::future::Future;
use embedded_io_async as io_async;
use heapless::String;
use static_cell::StaticCell;
use crate::round_buffer::RoundBuffer;
use embedded_graphics as eg;
use embedded_hal_async as hal_async;
use embedded_keycode as keycode;
use unicode_width::UnicodeWidthStr;
use crate::char_attr;
use crate::utf8_accumulator::Utf8Accumulator;
use crate::console_output::Config as ConsoleConfig;
use crate::graphics_output::Config as GraphicsConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    None,
    Entered,
    ReqCompletionFirst,
    ReqCompletionPrev,
    ReqCompletionNext,
}

pub trait Output {
    type Error;

    fn init(&mut self) -> impl Future<Output = ()>;
    fn write(&mut self, data: &[u8]) -> impl Future<Output = ()>;
    fn print(&mut self, s: &str) -> impl Future<Output = ()>;
    fn new_line(&mut self, line_buf_combined: &[u8]) -> impl Future<Output = ()>; 
    fn move_cursor_left_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn move_cursor_right_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn delete_to_line_end(&mut self) -> impl Future<Output = ()>;
    fn blink_cursor(&mut self) -> impl Future<Output = ()> {
        core::future::pending()
    }
    fn query_text_area_size(&mut self) -> impl Future<Output = ()>;
    fn update_text_area_size(&mut self, n_cols: usize, n_rows: usize) -> impl Future<Output = ()>;
}

pub struct DummyOutput;

impl Output for DummyOutput {
    type Error = ();

    async fn init(&mut self) {}
    async fn write(&mut self, _data: &[u8]) {}
    async fn print(&mut self, _s: &str) {}
    async fn new_line(&mut self, _line_buf_combined: &[u8]) {}
    async fn move_cursor_left_n(&mut self, _n: usize) {}
    async fn move_cursor_right_n(&mut self, _n: usize) {}
    async fn delete_to_line_end(&mut self) {}
    async fn blink_cursor(&mut self) {}
    async fn query_text_area_size(&mut self) -> () {}
    async fn update_text_area_size(&mut self, _n_cols: usize, _n_rows: usize) {}
}

pub struct Editor<'a, OutputT, CharAttr> {
    line_buf_whole: &'a mut [u8],
    history_buf: RoundBuffer<'a>,
    offset_history: usize,
    output: OutputT,
    ibyte_cur: usize,
    ibyte_end: usize,
    ibyte_completion: usize,
    range_result: (usize, usize),
    first_of_line: bool,
    nbytes_prompt: usize,
    _phantom: core::marker::PhantomData<CharAttr>,
}

impl<'a, OutputT: Output, CharAttr: char_attr::CharAttr> Editor<'a, OutputT, CharAttr> {
    const IBYTE_NONE: usize = usize::MAX;

    pub fn new(line_buf_whole: &'a mut [u8], history_buf: &'a mut [u8], output: OutputT) -> Self {
        assert!(line_buf_whole.len() < history_buf.len(), "line buffer must be smaller than history buffer");
        Self {
            line_buf_whole,
            history_buf: RoundBuffer::new(history_buf),
            offset_history: RoundBuffer::OFFSET_NONE,
            output,
            ibyte_cur: 0,
            ibyte_end: 0,
            ibyte_completion: Self::IBYTE_NONE,
            range_result: (0, 0),
            first_of_line: true,
            nbytes_prompt: 0,
            _phantom: core::marker::PhantomData::<CharAttr>,
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        let nchars_prompt_new = prompt.chars().count();
        let nbytes_prompt_new = prompt.len() + nchars_prompt_new * CharAttr::NBYTES;
        if nbytes_prompt_new + self.ibyte_end > self.line_buf_whole.len() { return; }
        self.line_buf_whole.copy_within(self.nbytes_prompt..self.nbytes_prompt + self.ibyte_end, nbytes_prompt_new);
        let mut utf8_acc = Utf8Accumulator::new();
        let mut ibyte_dst = 0;
        let char_attr = CharAttr::new();
        for byte in prompt.as_bytes() {
            if utf8_acc.push(*byte) {
                self.line_buf_whole[ibyte_dst..ibyte_dst + CharAttr::NBYTES]
                    .copy_from_slice(char_attr.get_bytes());
                ibyte_dst += CharAttr::NBYTES;
                let s = utf8_acc.get_result();
                self.line_buf_whole[ibyte_dst..ibyte_dst + s.len()].copy_from_slice(s.as_bytes());
                ibyte_dst += s.len();
                utf8_acc.reset();
            }
        }
        self.nbytes_prompt = nbytes_prompt_new;
    }

    pub async fn init(&mut self) {
        self.ibyte_cur = 0;
        self.ibyte_end = 0;
        self.ibyte_completion = Self::IBYTE_NONE;
        self.range_result = (0, 0);
        self.first_of_line = true;
        self.output.init().await;
        self.output.query_text_area_size().await;
    }

    pub async fn process_key(&mut self, key: serkey::Key) -> Event {
        //println!("Received key: {:?}", key);
        let mut event = Event::None;
        let mut history_operation = false;
        let mut quit_completion = true;
        self.range_result = (0, 0);
        match key {
            serkey::Key::CookedChar(c) => { self.insert_char(c).await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::A) => { self.move_cursor_to_line_start().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::B) => { self.move_cursor_left().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::D) => { self.delete_char().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::E) => { self.move_cursor_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::F) => { self.move_cursor_right().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::H) => { self.backspace().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::I) => {
                event = self.request_completition(false).await; quit_completion = false; }
            serkey::Key::CookedCtrl(serkey::Ctrl::J) => { self.enter().await; event = Event::Entered; }
            serkey::Key::CookedCtrl(serkey::Ctrl::K) => { self.delete_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::N) => {
                self.recall_history_next().await; history_operation = true; }
            serkey::Key::CookedCtrl(serkey::Ctrl::P) => {
                self.recall_history_prev().await; history_operation = true; }
            serkey::Key::TextAreaSizeReport(n_cols, n_rows) => {
                self.output.update_text_area_size(n_cols as usize, n_rows as usize).await;
            }
            serkey::Key::A(attr) if attr.is_ctrl() => { self.move_cursor_to_line_start().await; }
            serkey::Key::B(attr) if attr.is_ctrl() => { self.move_cursor_left().await; }
            serkey::Key::D(attr) if attr.is_ctrl() => { self.delete_char().await; }
            serkey::Key::E(attr) if attr.is_ctrl() => { self.move_cursor_to_line_end().await; }
            serkey::Key::F(attr) if attr.is_ctrl() => { self.move_cursor_right().await; }
            serkey::Key::H(attr) if attr.is_ctrl() => { self.backspace().await; }
            serkey::Key::J(attr) if attr.is_ctrl() => { self.enter().await; event = Event::Entered; }
            serkey::Key::K(attr) if attr.is_ctrl() => { self.delete_to_line_end().await; }
            serkey::Key::N(attr) if attr.is_ctrl() => {
                self.recall_history_next().await; history_operation = true; }
            serkey::Key::P(attr) if attr.is_ctrl() => {
                self.recall_history_prev().await; history_operation = true; }
            serkey::Key::Enter(attr) if attr.is_none() => { self.enter().await; event = Event::Entered; }
            serkey::Key::Tab(attr) if attr.is_none() => {
                event = self.request_completition(false).await; quit_completion = false; }
            serkey::Key::Tab(attr) if attr.is_shift() => {
                event = self.request_completition(true).await; quit_completion = false; }
            serkey::Key::Delete(attr) if attr.is_none() => { self.delete_char().await; }
            serkey::Key::Backspace(attr) if attr.is_none() => { self.backspace().await; }
            serkey::Key::Up(attr) if attr.is_none() => {
                self.recall_history_prev().await; history_operation = true; }
            serkey::Key::Down(attr) if attr.is_none() => {
                self.recall_history_next().await; history_operation = true; }
            serkey::Key::Left(attr) if attr.is_none() => { self.move_cursor_left().await; }
            serkey::Key::Right(attr) if attr.is_none() => { self.move_cursor_right().await; }
            serkey::Key::Home(attr) if attr.is_none() => { self.move_cursor_to_line_start().await; }
            serkey::Key::End(attr) if attr.is_none() => { self.move_cursor_to_line_end().await; }
            _ => {}
        }
        if !history_operation { self.offset_history = RoundBuffer::OFFSET_NONE; }
        if quit_completion { self.ibyte_completion = Self::IBYTE_NONE; }
        event
    }

    pub async fn read_key<ReadKey>(&mut self, reader: &mut ReadKey) -> Result<serkey::Key, ReadKey::Error>
    where ReadKey: keycode::ReadKey
    {
        if self.first_of_line {
            self.first_of_line = false;
            let prompt = &self.line_buf_whole[..self.nbytes_prompt];
            self.output.write(prompt).await;
        }
        let mut read = core::pin::pin!(reader.read_key());
        loop {
            let mut blink = core::pin::pin!(self.output.blink_cursor());
            let result = core::future::poll_fn(|cx| {
                if let core::task::Poll::Ready(result) = read.as_mut().poll(cx) {
                    return core::task::Poll::Ready(Some(result));
                }
                if blink.as_mut().poll(cx).is_ready() {
                    return core::task::Poll::Ready(None);
                }
                core::task::Poll::Pending
            }).await;
            if let Some(result) = result { break result; }
        }
    }

    async fn request_completition(&mut self, reverse: bool) -> Event {
        if self.ibyte_completion == Self::IBYTE_NONE {
            self.move_cursor_to_line_end().await;
            self.ibyte_completion = self.ibyte_cur;
            Event::ReqCompletionFirst
        } else if reverse {
            Event::ReqCompletionPrev
        } else {
            Event::ReqCompletionNext
        }
    }

    pub fn get_result(&self) -> &[u8] {
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let (ibyte_start, ibyte_end) = self.range_result;
        let nbytes_copy = (ibyte_end - ibyte_start).min(line_buf.len());
        &line_buf[ibyte_start..ibyte_start + nbytes_copy]
    }

    pub fn read_result_str<const N: usize>(&self, result_str: &mut String::<N>) -> Result<(), heapless::CapacityError> {
        result_str.clear();
        let mut utf8_acc = Utf8Accumulator::new();
        let mut ibyte_char_attr = 0;
        for byte in self.get_result() {
            if ibyte_char_attr < CharAttr::NBYTES {
                ibyte_char_attr += 1;
            } else if utf8_acc.push(*byte) {
                let s = utf8_acc.get_result();
                result_str.push_str(s).unwrap();
                utf8_acc.reset();
                ibyte_char_attr = 0;
            }
        }
        Ok(())
    }

    pub async fn print(&mut self, s: &str) {
        self.output.print(s).await;
    }

    pub async fn enter(&mut self) {
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let (ibyte_start, ibyte_end) = Self::strip(&line_buf[..self.ibyte_end]);
        let line_buf_stripped = &line_buf[ibyte_start..ibyte_end];
        if ibyte_start != ibyte_end {
            let mut skip_push = false;
            let mut offset_seek = RoundBuffer::OFFSET_NONE;
            if self.history_buf.seek_item_backward(&mut offset_seek) {
                let (first, second) = self.history_buf.get_item(offset_seek).unwrap();
                skip_push =
                    self.ibyte_end == first.len() + second.len() &&
                    line_buf_stripped[..first.len()].cmp(first) == core::cmp::Ordering::Equal &&
                    line_buf_stripped[first.len()..self.ibyte_end].cmp(second) == core::cmp::Ordering::Equal;
            }
            if !skip_push {
                self.history_buf.push(&line_buf_stripped).unwrap();
            }
        }
        self.range_result = (ibyte_start, ibyte_end);
        self.output.new_line(&self.line_buf_whole[..self.nbytes_prompt + self.ibyte_end]).await;
        self.ibyte_cur = 0;
        self.ibyte_end = 0;
        self.first_of_line = true;
    }

    pub async fn recall_history_prev(&mut self) {
        let width = self.calc_width(0, self.ibyte_cur);
        if self.history_buf.seek_item_backward(&mut self.offset_history) {
            let (first, second) = self.history_buf.get_item(self.offset_history).unwrap();
            let ibyte_end = first.len() + second.len();
            let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
            assert!(ibyte_end <= line_buf.len());
            line_buf[..first.len()].copy_from_slice(first);
            line_buf[first.len()..ibyte_end].copy_from_slice(second);
            self.ibyte_end = ibyte_end;
            self.ibyte_cur = ibyte_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&line_buf[..ibyte_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn recall_history_next(&mut self) {
        let width = self.calc_width(0, self.ibyte_cur);
        if self.history_buf.seek_item_forward(&mut self.offset_history) {
            let (first, second) = self.history_buf.get_item(self.offset_history).unwrap();
            let ibyte_end = first.len() + second.len();
            let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
            assert!(ibyte_end <= line_buf.len());
            line_buf[..first.len()].copy_from_slice(first);
            line_buf[first.len()..ibyte_end].copy_from_slice(second);
            self.ibyte_end = ibyte_end;
            self.ibyte_cur = ibyte_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&line_buf[..ibyte_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn fulfill_completion(&mut self, s: &str) {
        if self.ibyte_completion == Self::IBYTE_NONE { return; }
        let nbytes_inserted = s.len() + s.chars().count() * CharAttr::NBYTES;
        let ibyte_end = self.ibyte_completion + nbytes_inserted;
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        if ibyte_end > line_buf.len() { return; }
        let width = self.calc_width(self.ibyte_completion, self.ibyte_end);
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        let mut utf8_acc = Utf8Accumulator::new();
        let mut ibyte_dst = self.ibyte_completion;
        let char_attr = CharAttr::new();
        for byte in s.as_bytes() {
            if utf8_acc.push(*byte) {
                line_buf[ibyte_dst..ibyte_dst + CharAttr::NBYTES].copy_from_slice(char_attr.get_bytes());
                ibyte_dst += CharAttr::NBYTES;
                let s = utf8_acc.get_result();
                line_buf[ibyte_dst..ibyte_dst + s.len()].copy_from_slice(s.as_bytes());
                ibyte_dst += s.len();
                utf8_acc.reset();
            }
        }
        self.ibyte_end = ibyte_end;
        self.ibyte_cur = ibyte_end;
        self.output.move_cursor_left_n(width).await;
        self.output.write(&line_buf[self.ibyte_completion..ibyte_end]).await;
        self.output.delete_to_line_end().await;
    }

    pub async fn insert_char(&mut self, c: char) {
        let nbytes_inserted = self.insert_char_to_buf(self.ibyte_cur, c);
        if nbytes_inserted == 0 { return; }
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let ibyte_next = self.ibyte_cur + nbytes_inserted;
        if self.ibyte_end < ibyte_next { self.ibyte_end = ibyte_next; }
        self.output.write(&line_buf[self.ibyte_cur..self.ibyte_end]).await;
        if ibyte_next < self.ibyte_end {
            self.output.move_cursor_left_n(self.calc_width(ibyte_next, self.ibyte_end)).await;
        }
        self.ibyte_cur = ibyte_next;
    }

    pub async fn delete_char(&mut self) {
        if self.ibyte_cur >= self.ibyte_end { return; }
        self.delete_char_from_buf(self.ibyte_cur);
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        self.output.write(&line_buf[self.ibyte_cur..self.ibyte_end]).await;
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
        let mut ibyte_prev = self.ibyte_cur - 1;
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        while ibyte_prev > 0 && line_buf[ibyte_prev] & 0xc0 == 0x80 {
            ibyte_prev -= 1;
        }
        ibyte_prev -= CharAttr::NBYTES;
        self.output.move_cursor_left_n(self.calc_width(ibyte_prev, self.ibyte_cur)).await;
        self.ibyte_cur = ibyte_prev;
        true
    }

    pub async fn move_cursor_right(&mut self) -> bool {
        if self.ibyte_cur >= self.ibyte_end { return false; }
        let mut ibyte_next = self.ibyte_cur + CharAttr::NBYTES + 1;
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        while ibyte_next < self.ibyte_end && line_buf[ibyte_next] & 0xc0 == 0x80 {
            ibyte_next += 1;
        }
        self.output.move_cursor_right_n(self.calc_width(self.ibyte_cur, ibyte_next)).await;
        self.ibyte_cur = ibyte_next;
        true
    }

    pub fn insert_char_to_buf(&mut self, ibyte: usize, c: char) -> usize {
        let mut buf_enc = [0u8; 4];
        let bytes_insert = c.encode_utf8(&mut buf_enc).as_bytes();
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        if self.ibyte_end + CharAttr::NBYTES + bytes_insert.len() > line_buf.len() { return 0; }
        if ibyte < self.ibyte_end {
            line_buf.copy_within(ibyte..self.ibyte_end, ibyte + bytes_insert.len());
        }
        let mut char_attr = CharAttr::new();
        //char_attr.set_color_fg(Color::RED);
        line_buf[ibyte..ibyte + CharAttr::NBYTES].copy_from_slice(&char_attr.get_bytes());
        line_buf[ibyte + CharAttr::NBYTES..ibyte + CharAttr::NBYTES + bytes_insert.len()].copy_from_slice(bytes_insert);
        self.ibyte_end += CharAttr::NBYTES + bytes_insert.len();
        CharAttr::NBYTES + bytes_insert.len()
    }

    pub fn delete_char_from_buf(&mut self, ibyte: usize) -> usize {
        if ibyte >= self.ibyte_end { return 0; }
        let mut nbytes_delete = CharAttr::NBYTES + 1;
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        while ibyte + nbytes_delete < self.ibyte_end && line_buf[ibyte + nbytes_delete] & 0xc0 == 0x80 {
            nbytes_delete += 1;
        }
        line_buf.copy_within(ibyte + nbytes_delete..self.ibyte_end, ibyte);
        self.ibyte_end -= nbytes_delete;
        nbytes_delete
    }

    pub fn calc_width(&self, ibyte_start: usize, ibyte_end: usize) -> usize {
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let mut utf8_acc = Utf8Accumulator::new();
        let mut ibyte_char_attr = 0;
        let mut width = 0;
        for byte in &line_buf[ibyte_start..ibyte_end] {
            if ibyte_char_attr < CharAttr::NBYTES {
                ibyte_char_attr += 1;
            } else if utf8_acc.push(*byte) {
                let s = utf8_acc.get_result();
                width += UnicodeWidthStr::width(s);
                utf8_acc.reset();
                ibyte_char_attr = 0;
            }
        }
        width
    }

    fn strip(buf: &[u8]) -> (usize, usize) {
        fn is_skippable(byte: u8) -> bool {
            byte == 0x00 || byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r'
        }
        let ibyte_start = {
            let mut ibyte = 0;
            while ibyte + CharAttr::NBYTES < buf.len() && is_skippable(buf[ibyte + CharAttr::NBYTES]) {
                ibyte += CharAttr::NBYTES + 1;
            }
            ibyte
        };
        let ibyte_end = {
            let mut ibyte = buf.len();
            while ibyte >= CharAttr::NBYTES + 1 && is_skippable(buf[ibyte - 1]) {
                ibyte -= CharAttr::NBYTES + 1;
            }
            ibyte
        };
        (ibyte_start, ibyte_end)
    }
}

#[allow(dead_code)]
pub async fn simple_task_console<Writer, KeyReader>(writer: Writer, key_reader: KeyReader, enable_break: bool)
where
    Writer: io_async::Write,
    KeyReader: keycode::ReadKey,
{
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
    
    let config = ConsoleConfig::default();
    let editor = Editor::with_console_mono_color(line_buf, history_buf, writer, config);
    simple_task_common(editor, key_reader, enable_break).await;
}

#[allow(dead_code)]
pub async fn simple_task_graphics<DrawTarget, ColorType, KeyReader, DelayNs, FnRefresh>
    (target: DrawTarget, key_reader: KeyReader, enable_break: bool, delay: DelayNs, fn_refresh: FnRefresh)
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    ColorType: From<eg::pixelcolor::Rgb888>,
    KeyReader: keycode::ReadKey,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
{
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
    let rollback_buf = {
        static STATIC_CELL: StaticCell<[u8; 1024]> = StaticCell::new();
        STATIC_CELL.init([0u8; 1024])
    };
    let work_buf = {
        static STATIC_CELL: StaticCell<[u8; 128]> = StaticCell::new();
        STATIC_CELL.init([0u8; 128])
    };
    let config = GraphicsConfig::<ColorType>::default()
        .font(&eg::mono_font::ascii::FONT_10X20)
        .text_color(ColorType::from(eg::pixelcolor::Rgb888::new(255, 255, 255)))
        .background_color(ColorType::from(eg::pixelcolor::Rgb888::new(0, 0, 128)))
        .ht_cursor(2);
    let editor = Editor::with_graphics_mono_color(line_buf, history_buf, rollback_buf, work_buf, target, config, delay, fn_refresh);
    simple_task_common(editor, key_reader, enable_break).await;
}

pub async fn simple_task_common<OutputT, CharAttrType, KeyReader>(mut editor: Editor<'_, OutputT, CharAttrType>, mut reader: KeyReader, enable_break: bool)
where
    OutputT: Output,
    CharAttrType: char_attr::CharAttr,
    KeyReader: keycode::ReadKey,
{
    let mut sbuf_result: heapless::String<128> = heapless::String::new();
    editor.init().await;
    editor.set_prompt("> ");
    let mut cnt_completition = 0;
    let mut sbuf_completion: heapless::String<128> = heapless::String::new();
    let mut sbuf = String::<128>::new();
    loop {
        let key = editor.read_key(&mut reader).await;
        if let Ok(key) = key {
            if enable_break {
                if let serkey::Key::CookedCtrl(serkey::Ctrl::C) = key { break; }
            }
            match editor.process_key(key).await {
                Event::Entered => {
                    editor.read_result_str(&mut sbuf_result).ok();
                    sbuf.clear();
                    write!(sbuf, "Result: {}\r\n", sbuf_result).ok();
                    editor.print(&sbuf).await;
                    //editor.print("Result: ").await;
                    //editor.print(&sbuf_result).await;
                    //editor.print("\r\n").await;
                }
                Event::ReqCompletionFirst => {
                    cnt_completition = 0;
                    sbuf_completion.clear();
                    write!(sbuf_completion, "{}", cnt_completition).ok();
                    editor.fulfill_completion(&sbuf_completion).await;
                }
                Event::ReqCompletionPrev => {
                    cnt_completition -= 1;
                    sbuf_completion.clear();
                    write!(sbuf_completion, "{}", cnt_completition).ok();
                    editor.fulfill_completion(&sbuf_completion).await;
                }
                Event::ReqCompletionNext => {
                    cnt_completition += 1;
                    sbuf_completion.clear();
                    write!(sbuf_completion, "{}", cnt_completition).ok();
                    editor.fulfill_completion(&sbuf_completion).await;
                }
                Event::None => {}
            }
        }
    }
}
