use core::fmt::Write as _;
use core::future::Future;
use embedded_io_async as io_async;
use heapless::String;
use static_cell::StaticCell;
use crate::round_buffer::RoundBuffer;
use crate::console_output::ConsoleOutput;
use crate::graphics_output::GraphicsOutput;
use embedded_graphics as eg;
use embedded_hal_async as hal_async;
use embedded_keycode as keycode;
use unicode_width::UnicodeWidthStr;
use crate::char_attr::CharAttr;
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

pub struct Editor<'a, Output> {
    line_buf_whole: &'a mut [u8],
    history_buf: RoundBuffer<'a>,
    offset_history: usize,
    output: Output,
    idx_cur: usize,
    idx_end: usize,
    idx_completion: usize,
    range_result: (usize, usize),
    first_of_line: bool,
    nbytes_prompt: usize,
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    const IDX_NONE: usize = usize::MAX;

    pub fn new(line_buf_whole: &'a mut [u8], history_buf: &'a mut [u8], output: OutputT) -> Self {
        assert!(line_buf_whole.len() < history_buf.len(), "line buffer must be smaller than history buffer");
        Self {
            line_buf_whole,
            history_buf: RoundBuffer::new(history_buf),
            offset_history: RoundBuffer::OFFSET_NONE,
            output,
            idx_cur: 0,
            idx_end: 0,
            idx_completion: Self::IDX_NONE,
            range_result: (0, 0),
            first_of_line: true,
            nbytes_prompt: 0,
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        let nchars_prompt_new = prompt.chars().count();
        let nbytes_prompt_new = prompt.len() + nchars_prompt_new * CharAttr::NBYTES;
        if nbytes_prompt_new + self.idx_end > self.line_buf_whole.len() { return; }
        self.line_buf_whole.copy_within(self.nbytes_prompt..self.nbytes_prompt + self.idx_end, nbytes_prompt_new);
        let mut utf8_acc = Utf8Accumulator::new();
        let mut ibyte_dst = 0;
        for byte in prompt.as_bytes() {
            if utf8_acc.push(*byte) {
                self.line_buf_whole[ibyte_dst..ibyte_dst + CharAttr::NBYTES].fill(0xff);
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
        self.idx_cur = 0;
        self.idx_end = 0;
        self.idx_completion = Self::IDX_NONE;
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
        if quit_completion { self.idx_completion = Self::IDX_NONE; }
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
        if self.idx_completion == Self::IDX_NONE {
            self.move_cursor_to_line_end().await;
            self.idx_completion = self.idx_cur;
            Event::ReqCompletionFirst
        } else if reverse {
            Event::ReqCompletionPrev
        } else {
            Event::ReqCompletionNext
        }
    }

    pub fn get_result(&self) -> &[u8] {
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let (idx_start, idx_end) = self.range_result;
        let nbytes_copy = (idx_end - idx_start).min(line_buf.len());
        &line_buf[idx_start..idx_start + nbytes_copy]
    }

    pub fn read_result_str<const N: usize>(&self, result_str: &mut String::<N>) -> Result<(), heapless::CapacityError> {
        result_str.clear();
        let mut utf8_acc = Utf8Accumulator::new();
        let mut char_attr = CharAttr::new();
        for byte in self.get_result() {
            if !char_attr.is_complete() {
                char_attr.push(*byte);
            } else if utf8_acc.push(*byte) {
                let s = utf8_acc.get_result();
                result_str.push_str(s).unwrap();
                utf8_acc.reset();
                char_attr.clear();
            }
        }
        Ok(())
    }

    pub async fn print(&mut self, s: &str) {
        self.output.print(s).await;
    }

    pub async fn enter(&mut self) {
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let (idx_start, idx_end) = Self::strip(&line_buf[..self.idx_end]);
        let line_buf_stripped = &line_buf[idx_start..idx_end];
        if idx_start != idx_end {
            let mut skip_push = false;
            let mut offset_seek = RoundBuffer::OFFSET_NONE;
            if self.history_buf.seek_item_backward(&mut offset_seek) {
                let (first, second) = self.history_buf.get_item(offset_seek).unwrap();
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
        self.output.new_line(&self.line_buf_whole[..self.nbytes_prompt + self.idx_end]).await;
        self.idx_cur = 0;
        self.idx_end = 0;
        self.first_of_line = true;
    }

    pub async fn recall_history_prev(&mut self) {
        let width = self.calc_width(0, self.idx_cur);
        if self.history_buf.seek_item_backward(&mut self.offset_history) {
            let (first, second) = self.history_buf.get_item(self.offset_history).unwrap();
            let idx_end = first.len() + second.len();
            let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
            assert!(idx_end <= line_buf.len());
            line_buf[..first.len()].copy_from_slice(first);
            line_buf[first.len()..idx_end].copy_from_slice(second);
            self.idx_end = idx_end;
            self.idx_cur = idx_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&line_buf[..idx_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn recall_history_next(&mut self) {
        let width = self.calc_width(0, self.idx_cur);
        if self.history_buf.seek_item_forward(&mut self.offset_history) {
            let (first, second) = self.history_buf.get_item(self.offset_history).unwrap();
            let idx_end = first.len() + second.len();
            let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
            assert!(idx_end <= line_buf.len());
            line_buf[..first.len()].copy_from_slice(first);
            line_buf[first.len()..idx_end].copy_from_slice(second);
            self.idx_end = idx_end;
            self.idx_cur = idx_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&line_buf[..idx_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn fulfill_completion(&mut self, s: &str) {
        if self.idx_completion == Self::IDX_NONE { return; }
        let idx_end = self.idx_completion + s.len();
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        if idx_end > line_buf.len() { return; }
        let width = self.calc_width(self.idx_completion, self.idx_end);
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        //*****************************
        line_buf[self.idx_completion..idx_end].copy_from_slice(s.as_bytes());
        //*****************************
        self.idx_end = idx_end;
        self.idx_cur = idx_end;
        self.output.move_cursor_left_n(width).await;
        self.output.write(&line_buf[self.idx_completion..idx_end]).await;
        self.output.delete_to_line_end().await;
    }

    pub async fn insert_char(&mut self, c: char) {
        let nbytes_inserted = self.insert_char_to_buf(self.idx_cur, c);
        if nbytes_inserted == 0 { return; }
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let idx_next = self.idx_cur + nbytes_inserted;
        if self.idx_end < idx_next { self.idx_end = idx_next; }
        self.output.write(&line_buf[self.idx_cur..self.idx_end]).await;
        if idx_next < self.idx_end {
            self.output.move_cursor_left_n(self.calc_width(idx_next, self.idx_end)).await;
        }
        self.idx_cur = idx_next;
    }

    pub async fn delete_char(&mut self) {
        if self.idx_cur >= self.idx_end { return; }
        self.delete_char_from_buf(self.idx_cur);
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        self.output.write(&line_buf[self.idx_cur..self.idx_end]).await;
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
        let mut idx_prev = self.idx_cur - 1;
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        while idx_prev > 0 && line_buf[idx_prev] & 0xc0 == 0x80 {
            idx_prev -= 1;
        }
        idx_prev -= CharAttr::NBYTES;
        self.output.move_cursor_left_n(self.calc_width(idx_prev, self.idx_cur)).await;
        self.idx_cur = idx_prev;
        true
    }

    pub async fn move_cursor_right(&mut self) -> bool {
        if self.idx_cur >= self.idx_end { return false; }
        let mut idx_next = self.idx_cur + CharAttr::NBYTES + 1;
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        while idx_next < self.idx_end && line_buf[idx_next] & 0xc0 == 0x80 {
            idx_next += 1;
        }
        self.output.move_cursor_right_n(self.calc_width(self.idx_cur, idx_next)).await;
        self.idx_cur = idx_next;
        true
    }

    pub fn insert_char_to_buf(&mut self, idx: usize, c: char) -> usize {
        let mut buf_enc = [0u8; 4];
        let bytes_insert = c.encode_utf8(&mut buf_enc).as_bytes();
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        if self.idx_end + CharAttr::NBYTES + bytes_insert.len() > line_buf.len() { return 0; }
        if idx < self.idx_end {
            line_buf.copy_within(idx..self.idx_end, idx + bytes_insert.len());
        }
        line_buf[idx..idx + CharAttr::NBYTES].fill(0xff);
        line_buf[idx + CharAttr::NBYTES..idx + CharAttr::NBYTES + bytes_insert.len()].copy_from_slice(bytes_insert);
        self.idx_end += CharAttr::NBYTES + bytes_insert.len();
        CharAttr::NBYTES + bytes_insert.len()
    }

    pub fn delete_char_from_buf(&mut self, idx: usize) -> usize {
        if idx >= self.idx_end { return 0; }
        let mut nbytes_delete = CharAttr::NBYTES + 1;
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        while idx + nbytes_delete < self.idx_end && line_buf[idx + nbytes_delete] & 0xc0 == 0x80 {
            nbytes_delete += 1;
        }
        line_buf.copy_within(idx + nbytes_delete..self.idx_end, idx);
        self.idx_end -= nbytes_delete;
        nbytes_delete
    }

    pub fn calc_width(&self, idx_start: usize, idx_end: usize) -> usize {
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let mut utf8_acc = Utf8Accumulator::new();
        let mut char_attr = CharAttr::new();
        let mut width = 0;
        for byte in &line_buf[idx_start..idx_end] {
            if !char_attr.is_complete() {
                char_attr.push(*byte);
            } else if utf8_acc.push(*byte) {
                let s = utf8_acc.get_result();
                width += UnicodeWidthStr::width(s);
                utf8_acc.reset();
                char_attr.clear();
            }
        }
        width
    }

    fn strip(buf: &[u8]) -> (usize, usize) {
        fn is_skippable(byte: u8) -> bool {
            byte == 0x00 || byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r'
        }
        let idx_start = {
            let mut idx = 0;
            while idx + CharAttr::NBYTES < buf.len() && is_skippable(buf[idx + CharAttr::NBYTES]) {
                idx += CharAttr::NBYTES + 1;
            }
            idx
        };
        let idx_end = {
            let mut idx = buf.len();
            while idx >= CharAttr::NBYTES + 1 && is_skippable(buf[idx - 1]) {
                idx -= CharAttr::NBYTES + 1;
            }
            idx
        };
        (idx_start, idx_end)
    }
}

impl<'a, Writer: io_async::Write> Editor<'a, ConsoleOutput<Writer>> {
    pub fn with_serial_output(line_buf: &'a mut [u8], history_buf: &'a mut [u8], writer: Writer, config: ConsoleConfig) -> Self {
        Self::new(line_buf, history_buf, ConsoleOutput::new(writer, config))
    }
}

impl<'a, DrawTarget, DelayNs, FnRefresh, ColorType> Editor<'a, GraphicsOutput<'a, DrawTarget, DelayNs, FnRefresh>>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
    ColorType: From<eg::pixelcolor::Rgb888>,
{
    pub fn with_graphics_output(line_buf: &'a mut [u8], history_buf: &'a mut [u8],
        rollback_buf: &'a mut [u8], work_buf: &'a mut [u8], 
        target: DrawTarget, config: GraphicsConfig<ColorType>, delay: DelayNs, fn_refresh: FnRefresh) -> Self {
        Self::new(line_buf, history_buf, GraphicsOutput::new(target, rollback_buf, work_buf, config, delay, fn_refresh))
    }
}

#[allow(dead_code)]
pub async fn simple_task_stream(writer: impl io_async::Write, key_reader: impl keycode::ReadKey, enable_break: bool) {
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
    let editor = Editor::with_serial_output(line_buf, history_buf, writer, config);
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
    let editor = Editor::with_graphics_output(line_buf, history_buf, rollback_buf, work_buf, target, config, delay, fn_refresh);
    simple_task_common(editor, key_reader, enable_break).await;
}

pub async fn simple_task_common(mut editor: Editor<'_, impl Output>, mut reader: impl keycode::ReadKey, enable_break: bool) {
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
