use core::fmt::Write as _;
use core::future::Future;
use embedded_io_async as io_async;
use heapless::String;
use static_cell::StaticCell;
use crate::round_buffer::RoundBuffer;
use crate::stream_output::SerialOutput;
use crate::graphics_output::GraphicsOutput;
use embedded_graphics as eg;
use embedded_hal_async as hal_async;
use embedded_keycode as keycode;
pub use crate::graphics_output::Config as GraphicsConfig;

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
    fn new_line(&mut self) -> impl Future<Output = ()>; 
    fn move_cursor_left_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn move_cursor_right_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn delete_to_line_end(&mut self) -> impl Future<Output = ()>;
    fn blink_cursor(&mut self) -> impl Future<Output = ()> {
        core::future::pending()
    }
}

pub struct DummyOutput;

impl Output for DummyOutput {
    type Error = ();

    async fn init(&mut self) {}
    async fn write(&mut self, _data: &[u8]) {}
    async fn new_line(&mut self) {}
    async fn move_cursor_left_n(&mut self, _n: usize) {}
    async fn move_cursor_right_n(&mut self, _n: usize) {}
    async fn delete_to_line_end(&mut self) {}
}

pub struct Editor<'a, Output> {
    line_buf: &'a mut [u8],
    history_buf: RoundBuffer<'a>,
    output: Output,
    idx_cur: usize,
    idx_end: usize,
    idx_completion: usize,
    range_result: (usize, usize),
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    const IDX_NONE: usize = usize::MAX;

    pub fn new(line_buf: &'a mut [u8], history_buf: &'a mut [u8], output: OutputT) -> Self {
        assert!(line_buf.len() < history_buf.len(), "line buffer must be smaller than history buffer");
        Self {
            line_buf,
            history_buf: RoundBuffer::new(history_buf),
            output,
            idx_cur: 0,
            idx_end: 0,
            idx_completion: Self::IDX_NONE,
            range_result: (0, 0),
        }
    }

    pub async fn init(&mut self) {
        self.idx_cur = 0;
        self.idx_end = 0;
        self.idx_completion = Self::IDX_NONE;
        self.range_result = (0, 0);
        self.output.init().await;
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
        if !history_operation { self.history_buf.reset_cursor(); }
        if quit_completion { self.idx_completion = Self::IDX_NONE; }
        event
    }

    pub async fn read_key<ReadKey>(&mut self, reader: &mut ReadKey) -> Result<serkey::Key, ReadKey::Error>
    where ReadKey: keycode::ReadKey
    {
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
        let (idx_start, idx_end) = self.range_result;
        let nbytes_copy = (idx_end - idx_start).min(self.line_buf.len());
        &self.line_buf[idx_start..idx_start + nbytes_copy]
    }

    pub fn get_result_str(&self) -> &str {
        core::str::from_utf8(&self.get_result()).unwrap()
    }

    pub fn read_result_str<const N: usize>(&self, result_str: &mut String::<N>) -> Result<(), heapless::CapacityError> {
        result_str.clear();
        result_str.push_str(self.get_result_str())?;
        Ok(())
    }

    pub async fn print(&mut self, s: &str) {
        self.output.write(s.as_bytes()).await;
    }

    /// Waits for and performs the next cursor blink while the editor is idle.
    /// Custom input loops should poll this concurrently with their input future.
    //pub async fn wait_cursor_blink(&mut self) {
    //    self.output.wait_cursor_blink().await;
    //}

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

    pub async fn recall_history_prev(&mut self) {
        let width = self.calc_width(0, self.idx_cur);
        if let Some((first, second, _)) = self.history_buf.move_cursor_backward() {
            let idx_end = first.len() + second.len();
            assert!(idx_end <= self.line_buf.len());
            self.line_buf[..first.len()].copy_from_slice(first);
            self.line_buf[first.len()..idx_end].copy_from_slice(second);
            self.idx_end = idx_end;
            self.idx_cur = idx_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&self.line_buf[..idx_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn recall_history_next(&mut self) {
        let width = self.calc_width(0, self.idx_cur);
        if let Some((first, second, _)) = self.history_buf.move_cursor_forward() {
            let idx_end = first.len() + second.len();
            assert!(idx_end <= self.line_buf.len());
            self.line_buf[..first.len()].copy_from_slice(first);
            self.line_buf[first.len()..idx_end].copy_from_slice(second);
            self.idx_end = idx_end;
            self.idx_cur = idx_end;
            self.output.move_cursor_left_n(width).await;
            self.output.write(&self.line_buf[..idx_end]).await;
            self.output.delete_to_line_end().await;
        }
    }

    pub async fn fulfill_completion(&mut self, s: &str) {
        if self.idx_completion == Self::IDX_NONE { return; }
        let idx_end = self.idx_completion + s.len();
        if idx_end > self.line_buf.len() { return; }
        let width = self.calc_width(self.idx_completion, self.idx_end);
        self.line_buf[self.idx_completion..idx_end].copy_from_slice(s.as_bytes());
        self.idx_end = idx_end;
        self.idx_cur = idx_end;
        self.output.move_cursor_left_n(width).await;
        self.output.write(&self.line_buf[self.idx_completion..idx_end]).await;
        self.output.delete_to_line_end().await;
    }

    pub async fn insert_char(&mut self, c: char) {
        let nbytes = self.insert_char_to_buf(self.idx_cur, c);
        if nbytes == 0 { return; }
        let idx_next = self.idx_cur + nbytes;
        if self.idx_end < idx_next { self.idx_end = idx_next; }
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
}

impl<'a, Writer: io_async::Write> Editor<'a, SerialOutput<Writer>> {
    pub fn with_serial_output(line_buf: &'a mut [u8], history_buf: &'a mut [u8], writer: Writer) -> Self {
        Self::new(line_buf, history_buf, SerialOutput::new(writer))
    }
}

impl<'a, DrawTarget, DelayNs, FnRefresh, ColorType> Editor<'a, GraphicsOutput<DrawTarget, DelayNs, FnRefresh>>
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
    ColorType: eg::pixelcolor::RgbColor,
{
    pub fn with_graphics_output(
        line_buf: &'a mut [u8],
        history_buf: &'a mut [u8],
        target: DrawTarget,
        delay: DelayNs,
        config: GraphicsConfig,
        fn_refresh: FnRefresh,
    ) -> Self {
        Self::new(line_buf, history_buf, GraphicsOutput::new(target, delay, config, fn_refresh))
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
    let editor = Editor::with_serial_output(line_buf, history_buf, writer);
    simple_task_common(editor, key_reader, enable_break).await;
}

#[allow(dead_code)]
pub async fn simple_task_graphics<DrawTarget, ColorType, KeyReader, DelayNs, FnRefresh>
    (target: DrawTarget, key_reader: KeyReader, enable_break: bool, delay: DelayNs, fn_refresh: FnRefresh)
where
    DrawTarget: eg::draw_target::DrawTarget<Color = ColorType>,
    ColorType: eg::pixelcolor::RgbColor,
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
    let config = GraphicsConfig {
        cursor_blinking_interval: 500,
    };
    let editor = Editor::with_graphics_output(line_buf, history_buf, target, delay, config, fn_refresh);
    simple_task_common(editor, key_reader, enable_break).await;
}

pub async fn simple_task_common(mut editor: Editor<'_, impl Output>, mut reader: impl keycode::ReadKey, enable_break: bool) {
    let mut sbuf_result: heapless::String<128> = heapless::String::new();
    let prompt = "> ";
    editor.init().await;
    editor.print(prompt).await;
    let mut cnt_completition = 0;
    let mut sbuf_completion: heapless::String<128> = heapless::String::new();
    loop {
        //let key = {
        //    let mut read = core::pin::pin!(reader.read_key());
        //    loop {
        //        let mut blink = core::pin::pin!(editor.wait_cursor_blink());
        //        let result = core::future::poll_fn(|cx| {
        //            if let core::task::Poll::Ready(result) = read.as_mut().poll(cx) {
        //                return core::task::Poll::Ready(Some(result));
        //            }
        //            if blink.as_mut().poll(cx).is_ready() {
        //                return core::task::Poll::Ready(None);
        //            }
        //            core::task::Poll::Pending
        //        }).await;
        //        if let Some(result) = result { break result; }
        //    }
        //};
        let key = editor.read_key(&mut reader).await;
        if let Ok(key) = key {
            if enable_break {
                if let serkey::Key::CookedCtrl(serkey::Ctrl::C) = key { break; }
            }
            match editor.process_key(key).await {
                Event::Entered => {
                    editor.read_result_str(&mut sbuf_result).ok();
                    editor.print("Result: ").await;
                    editor.print(&sbuf_result).await;
                    editor.print("\r\n").await;
                    editor.print(prompt).await;
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
