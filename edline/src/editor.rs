use core::fmt::Write as _;
use core::future::Future;
use embedded_io_async as io_async;
use heapless::String;
use static_cell::StaticCell;
use embedded_graphics as eg;
use embedded_hal_async as hal_async;
use embedded_keycode as keycode;
use unicode_width::UnicodeWidthStr;
use crate::round_buffer::RoundBuffer;
use crate::round_buffer::RoundBufferMarker;
use crate::char_attr;
use crate::char_attr::CharAttr as _;
use crate::utf8_accumulator::Utf8Accumulator;
pub use crate::char_attr::{
    CharAttrMonoColor, CharAttrMonoColorDeco,
    CharAttr16Colors, CharAttr16ColorsDeco,
    CharAttr256Colors, CharAttr256ColorsDeco,
};
pub use crate::console_output::ConsoleConfig;
pub use crate::graphics_output::GraphicsConfig;

//-----------------------------------------------------------------------------
// Constructor functions
//-----------------------------------------------------------------------------
pub fn with_console<'a, CharAttr>(
    line_buf: &'a mut [u8],
    history_buf: &'a mut [u8],
    writer: impl io_async::Write,
    config: ConsoleConfig,
) -> Editor<'a, impl crate::editor::Output<CharAttr = CharAttr>>
where
    CharAttr: char_attr::CharAttr,
{
    let output = crate::console_output::ConsoleOutput::new(writer, config);
    Editor::new(line_buf, history_buf, output)
}

pub fn with_graphics<'a, CharAttr, DrawTarget, DelayNs, FnRefresh>(
    line_buf: &'a mut [u8],
    history_buf: &'a mut [u8],
    target: DrawTarget,
    rollback_buf: &'a mut [u8],
    config: GraphicsConfig<<DrawTarget as eg::draw_target::DrawTarget>::Color>,
    delay: DelayNs,
    fn_refresh: FnRefresh,
) -> Editor<'a, impl crate::editor::Output<CharAttr = CharAttr>>
where
    CharAttr: char_attr::CharAttr,
    DrawTarget: eg::draw_target::DrawTarget,
    DrawTarget::Color: From<eg::pixelcolor::Rgb888>,
    DelayNs: hal_async::delay::DelayNs,
    FnRefresh: FnMut(&mut DrawTarget),
{
    let output = crate::graphics_output::GraphicsOutput::new
        (target, rollback_buf, config, delay, fn_refresh);
    Editor::new(line_buf, history_buf, output)
}

//-----------------------------------------------------------------------------
// Event
//-----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    None,
    Entered,
    ReqCompletionFirst,
    ReqCompletionPrev,
    ReqCompletionNext,
}

//-----------------------------------------------------------------------------
// Output
//-----------------------------------------------------------------------------
pub trait Output {
    type Error;
    type CharAttr: char_attr::CharAttr;

    fn base_config(&self) -> &BaseConfig;
    fn init(&mut self) -> impl Future<Output = ()>;
    fn draw(&mut self, data: &[u8]) -> impl Future<Output = ()>;
    fn print(&mut self, s: &str) -> impl Future<Output = ()>;
    fn new_line(&mut self, line_buf_combined: &[u8]) -> impl Future<Output = ()>; 
    fn move_cursor_left_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn move_cursor_right_n(&mut self, n: usize) -> impl Future<Output = ()>;
    fn delete_to_line_end(&mut self) -> impl Future<Output = ()>;
    fn blink_cursor(&mut self) -> impl Future<Output = ()> { core::future::pending() }
    fn query_text_area_size(&mut self) -> impl Future<Output = ()>;
    fn update_text_area_size(&mut self, n_cols: usize, n_rows: usize) -> impl Future<Output = ()>;
}

//-----------------------------------------------------------------------------
// BaseConfig
//-----------------------------------------------------------------------------
pub struct BaseConfig {}

impl BaseConfig {
    pub fn default() -> Self {
        Self {}
    }
} 

//-----------------------------------------------------------------------------
// DummyOutput
//-----------------------------------------------------------------------------
pub struct DummyOutput {
    dummy_config: DummyConfig,
}

impl Output for DummyOutput {
    type Error = ();
    type CharAttr = char_attr::CharAttrMonoColor;

    fn base_config(&self) -> &BaseConfig { &self.dummy_config.base_config }
    async fn init(&mut self) {}
    async fn draw(&mut self, _data: &[u8]) {}
    async fn print(&mut self, _s: &str) {}
    async fn new_line(&mut self, _line_buf_combined: &[u8]) {}
    async fn move_cursor_left_n(&mut self, _n: usize) {}
    async fn move_cursor_right_n(&mut self, _n: usize) {}
    async fn delete_to_line_end(&mut self) {}
    async fn blink_cursor(&mut self) {}
    async fn query_text_area_size(&mut self) -> () {}
    async fn update_text_area_size(&mut self, _n_cols: usize, _n_rows: usize) {}
}

//-----------------------------------------------------------------------------
// DummyConfig
//-----------------------------------------------------------------------------
pub struct DummyConfig {
    pub base_config: BaseConfig,
}

impl DummyConfig {
    pub fn default() -> Self {
        Self { base_config: BaseConfig::default() }
    }
}

//-----------------------------------------------------------------------------
// Editor
//-----------------------------------------------------------------------------
pub struct Editor<'a, OutputT: Output> {
    line_buf_whole: &'a mut [u8],
    history_buf: RoundBuffer<'a>,
    marker_for_history: RoundBufferMarker,
    output: OutputT,
    ibyte_cur: usize,
    ibyte_end: usize,
    ibyte_completion: usize,
    range_result: (usize, usize),
    first_of_line: bool,
    nbytes_prompt: usize,
    char_attr_insert: OutputT::CharAttr,
    char_attr_completion: OutputT::CharAttr,
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    const IBYTE_NONE: usize = usize::MAX;

    pub fn new(line_buf_whole: &'a mut [u8], history_buf: &'a mut [u8], output: OutputT) -> Self {
        assert!(line_buf_whole.len() < history_buf.len(), "line buffer must be smaller than history buffer");
        Self {
            line_buf_whole,
            history_buf: RoundBuffer::new(history_buf),
            marker_for_history: RoundBufferMarker::new(),
            output,
            ibyte_cur: 0,
            ibyte_end: 0,
            ibyte_completion: Self::IBYTE_NONE,
            range_result: (0, 0),
            first_of_line: true,
            nbytes_prompt: 0,
            char_attr_insert: OutputT::CharAttr::new(),
            char_attr_completion: OutputT::CharAttr::new().set_underlined(),
        }
    }

    pub fn get_char_attr_insert(&self) -> OutputT::CharAttr {
        self.char_attr_insert
    }

    pub fn set_char_attr_insert(&mut self, char_attr: OutputT::CharAttr) {
        self.char_attr_insert = char_attr;
    }

    pub fn get_char_attr_completion(&self) -> OutputT::CharAttr {
        self.char_attr_completion
    }

    pub fn set_char_attr_completion(&mut self, char_attr: OutputT::CharAttr) {
        self.char_attr_completion = char_attr;
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        let nchars_prompt_new = prompt.chars().count();
        let nbytes_prompt_new = prompt.len() + nchars_prompt_new * OutputT::CharAttr::NBYTES;
        if nbytes_prompt_new + self.ibyte_end > self.line_buf_whole.len() { return; }
        self.line_buf_whole.copy_within(self.nbytes_prompt..self.nbytes_prompt + self.ibyte_end, nbytes_prompt_new);
        let mut utf8_acc = Utf8Accumulator::new();
        let mut ibyte_dst = 0;
        let char_attr = OutputT::CharAttr::new();
        for byte in prompt.as_bytes() {
            if utf8_acc.push(*byte) {
                self.line_buf_whole[ibyte_dst..ibyte_dst + OutputT::CharAttr::NBYTES]
                    .copy_from_slice(char_attr.get_bytes());
                ibyte_dst += OutputT::CharAttr::NBYTES;
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
        self.range_result = (0, 0);
        match key {
            serkey::Key::CookedChar(c) => {
                self.commit_completion().await; self.insert_char(c).await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::A) => {
                self.commit_completion().await; self.move_cursor_to_line_start().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::B) => {
                self.commit_completion().await; self.move_cursor_left().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::D) => {
                self.commit_completion().await; self.delete_char().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::E) => {
                self.commit_completion().await; self.move_cursor_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::F) => {
                self.commit_completion().await; self.move_cursor_right().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::H) => {
                self.commit_completion().await; self.backspace().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::I) => {
                event = self.request_completition_next().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::J) => {
                self.commit_completion().await; self.enter().await; event = Event::Entered; }
            serkey::Key::CookedCtrl(serkey::Ctrl::K) => {
                self.commit_completion().await; self.delete_to_line_end().await; }
            serkey::Key::CookedCtrl(serkey::Ctrl::N) => {
                self.cancel_completion_quick(); self.recall_history_next().await; history_operation = true; }
            serkey::Key::CookedCtrl(serkey::Ctrl::P) => {
                self.cancel_completion_quick(); self.recall_history_prev().await; history_operation = true; }
            serkey::Key::TextAreaSizeReport(n_cols, n_rows) => {
                self.output.update_text_area_size(n_cols as usize, n_rows as usize).await;
            }
            serkey::Key::A(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.move_cursor_to_line_start().await; }
            serkey::Key::B(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.move_cursor_left().await; }
            serkey::Key::D(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.delete_char().await; }
            serkey::Key::E(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.move_cursor_to_line_end().await; }
            serkey::Key::F(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.move_cursor_right().await; }
            serkey::Key::H(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.backspace().await; }
            serkey::Key::J(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.enter().await; event = Event::Entered; }
            serkey::Key::K(attr) if attr.is_ctrl() => {
                self.commit_completion().await; self.delete_to_line_end().await; }
            serkey::Key::N(attr) if attr.is_ctrl() => {
                self.cancel_completion_quick(); self.recall_history_next().await; history_operation = true; }
            serkey::Key::P(attr) if attr.is_ctrl() => {
                self.cancel_completion_quick(); self.recall_history_prev().await; history_operation = true; }
            serkey::Key::Enter(attr) if attr.is_none() => {
                self.commit_completion().await; self.enter().await; event = Event::Entered; }
            serkey::Key::Tab(attr) if attr.is_none() => {
                event = self.request_completition_next().await; }
            serkey::Key::Tab(attr) if attr.is_shift() => {
                event = self.request_completition_prev().await; }
            serkey::Key::Delete(attr) if attr.is_none() => {
                self.commit_completion().await; self.delete_char().await; }
            serkey::Key::Backspace(attr) if attr.is_none() => {
                self.commit_completion().await; self.backspace().await; }
            serkey::Key::Up(attr) if attr.is_none() => {
                self.cancel_completion_quick(); self.recall_history_prev().await; history_operation = true; }
            serkey::Key::Down(attr) if attr.is_none() => {
                self.cancel_completion_quick(); self.recall_history_next().await; history_operation = true; }
            serkey::Key::Left(attr) if attr.is_none() => {
                self.commit_completion().await; self.move_cursor_left().await; }
            serkey::Key::Right(attr) if attr.is_none() => {
                self.commit_completion().await; self.move_cursor_right().await; }
            serkey::Key::Home(attr) if attr.is_none() => {
                self.commit_completion().await; self.move_cursor_to_line_start().await; }
            serkey::Key::End(attr) if attr.is_none() => {
                self.commit_completion().await; self.move_cursor_to_line_end().await; }
            _ => {}
        }
        if !history_operation { self.marker_for_history = RoundBufferMarker::new(); }
        event
    }

    pub async fn read_key<ReadKey>(&mut self, reader: &mut ReadKey) -> Result<serkey::Key, ReadKey::Error>
    where ReadKey: keycode::ReadKey
    {
        if self.first_of_line {
            self.first_of_line = false;
            let prompt = &self.line_buf_whole[..self.nbytes_prompt];
            self.output.draw(prompt).await;
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

    async fn request_completition_next(&mut self) -> Event {
        if self.ibyte_completion == Self::IBYTE_NONE {
            self.move_cursor_to_line_end().await;
            self.ibyte_completion = self.ibyte_cur;
            Event::ReqCompletionFirst
        } else {
            Event::ReqCompletionNext
        }
    }

    async fn request_completition_prev(&mut self) -> Event {
        if self.ibyte_completion == Self::IBYTE_NONE {
            self.move_cursor_to_line_end().await;
            self.ibyte_completion = self.ibyte_cur;
            Event::ReqCompletionFirst
        } else {
            Event::ReqCompletionPrev
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
            if ibyte_char_attr < OutputT::CharAttr::NBYTES {
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
            let mut marker = RoundBufferMarker::new();
            if !(self.history_buf.seek_item_prev(&mut marker) &&
                    self.history_buf.is_item_equal(marker, line_buf_stripped)) {
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
        if !self.history_buf.seek_item_prev(&mut self.marker_for_history) { return; }
        self.recall_history().await;
    }

    pub async fn recall_history_next(&mut self) {
        if !self.history_buf.seek_item_next(&mut self.marker_for_history) { return; }
        self.recall_history().await;
    }

    pub async fn recall_history(&mut self) {
        let width = self.calc_width(0, self.ibyte_cur);
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        let nbytes = self.history_buf.read_item(self.marker_for_history, line_buf);
        self.ibyte_end = nbytes;
        self.ibyte_cur = nbytes;
        self.output.move_cursor_left_n(width).await;
        self.output.draw(&line_buf[..nbytes]).await;
        self.output.delete_to_line_end().await;
    }

    pub async fn fulfill_completion(&mut self, s: &str) {
        if self.ibyte_completion == Self::IBYTE_NONE { return; }
        let nbytes_inserted = s.len() + s.chars().count() * OutputT::CharAttr::NBYTES;
        let ibyte_end = self.ibyte_completion + nbytes_inserted;
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        if ibyte_end > line_buf.len() { return; }
        let width = self.calc_width(self.ibyte_completion, self.ibyte_end);
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        let mut utf8_acc = Utf8Accumulator::new();
        let mut ibyte_dst = self.ibyte_completion;
        for byte in s.as_bytes() {
            if utf8_acc.push(*byte) {
                line_buf[ibyte_dst..ibyte_dst + OutputT::CharAttr::NBYTES]
                    .copy_from_slice(self.char_attr_completion.get_bytes());
                ibyte_dst += OutputT::CharAttr::NBYTES;
                let s = utf8_acc.get_result();
                line_buf[ibyte_dst..ibyte_dst + s.len()].copy_from_slice(s.as_bytes());
                ibyte_dst += s.len();
                utf8_acc.reset();
            }
        }
        self.ibyte_end = ibyte_end;
        self.ibyte_cur = ibyte_end;
        self.output.move_cursor_left_n(width).await;
        self.output.draw(&line_buf[self.ibyte_completion..ibyte_end]).await;
        self.output.delete_to_line_end().await;
    }

    pub async fn commit_completion(&mut self) {
        if self.ibyte_completion == Self::IBYTE_NONE { return; }
        self.replace_char_attr(self.ibyte_completion, self.ibyte_end, self.char_attr_insert);
        let width = self.calc_width(self.ibyte_completion, self.ibyte_end);
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        self.output.move_cursor_left_n(width).await;
        self.output.draw(&line_buf[self.ibyte_completion..self.ibyte_end]).await;
        self.output.delete_to_line_end().await;
        self.ibyte_completion = Self::IBYTE_NONE;
    }

    pub async fn cancel_completion(&mut self) {
        if self.ibyte_completion == Self::IBYTE_NONE { return; }
        let width = self.calc_width(self.ibyte_completion, self.ibyte_end);
        self.output.move_cursor_left_n(width).await;
        self.output.delete_to_line_end().await;
        self.ibyte_end = self.ibyte_completion;
        self.ibyte_completion = Self::IBYTE_NONE;
    }

    pub fn cancel_completion_quick(&mut self) {
        if self.ibyte_completion == Self::IBYTE_NONE { return; }
        self.ibyte_end = self.ibyte_completion;
        self.ibyte_completion = Self::IBYTE_NONE;
    }

    fn replace_char_attr(&mut self, ibyte_start: usize, ibyte_end: usize, char_attr: OutputT::CharAttr) {
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        let mut ibyte = ibyte_start;
        while ibyte < ibyte_end {
            line_buf[ibyte..ibyte + OutputT::CharAttr::NBYTES].copy_from_slice(char_attr.get_bytes());
            ibyte += OutputT::CharAttr::NBYTES + 1;
            while ibyte < ibyte_end && line_buf[ibyte] & 0b1100_0000 == 0b1000_0000 {
                ibyte += 1;
            }
        }
    }

    pub async fn insert_char(&mut self, c: char) {
        let nbytes_inserted = self.insert_char_to_buf(self.ibyte_cur, c);
        if nbytes_inserted == 0 { return; }
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        let ibyte_next = self.ibyte_cur + nbytes_inserted;
        if self.ibyte_end < ibyte_next { self.ibyte_end = ibyte_next; }
        self.output.draw(&line_buf[self.ibyte_cur..self.ibyte_end]).await;
        if ibyte_next < self.ibyte_end {
            self.output.move_cursor_left_n(self.calc_width(ibyte_next, self.ibyte_end)).await;
        }
        self.ibyte_cur = ibyte_next;
    }

    pub async fn delete_char(&mut self) {
        if self.ibyte_cur >= self.ibyte_end { return; }
        self.delete_char_from_buf(self.ibyte_cur);
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        self.output.draw(&line_buf[self.ibyte_cur..self.ibyte_end]).await;
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
        while ibyte_prev > 0 && line_buf[ibyte_prev] & 0b1100_0000 == 0b1000_0000 {
            ibyte_prev -= 1;
        }
        ibyte_prev -= OutputT::CharAttr::NBYTES;
        self.output.move_cursor_left_n(self.calc_width(ibyte_prev, self.ibyte_cur)).await;
        self.ibyte_cur = ibyte_prev;
        true
    }

    pub async fn move_cursor_right(&mut self) -> bool {
        if self.ibyte_cur >= self.ibyte_end { return false; }
        let mut ibyte_next = self.ibyte_cur + OutputT::CharAttr::NBYTES + 1;
        let line_buf = &self.line_buf_whole[self.nbytes_prompt..];
        while ibyte_next < self.ibyte_end && line_buf[ibyte_next] & 0b1100_0000 == 0b1000_0000 {
            ibyte_next += 1;
        }
        self.output.move_cursor_right_n(self.calc_width(self.ibyte_cur, ibyte_next)).await;
        self.ibyte_cur = ibyte_next;
        true
    }

    pub fn insert_char_to_buf(&mut self, ibyte: usize, c: char) -> usize {
        let mut buf_enc = [0u8; 4];
        let bytes_char = c.encode_utf8(&mut buf_enc).as_bytes();
        let nbytes_insert = OutputT::CharAttr::NBYTES + bytes_char.len();
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        if self.ibyte_end + nbytes_insert > line_buf.len() { return 0; }
        if ibyte < self.ibyte_end {
            line_buf.copy_within(ibyte..self.ibyte_end, ibyte + nbytes_insert);
        }
        line_buf[ibyte..ibyte + OutputT::CharAttr::NBYTES].copy_from_slice(&self.char_attr_insert.get_bytes());
        line_buf[ibyte + OutputT::CharAttr::NBYTES..ibyte + nbytes_insert].copy_from_slice(bytes_char);
        self.ibyte_end += nbytes_insert;
        nbytes_insert
    }

    pub fn delete_char_from_buf(&mut self, ibyte: usize) -> usize {
        if ibyte >= self.ibyte_end { return 0; }
        let mut nbytes_delete = OutputT::CharAttr::NBYTES + 1;
        let line_buf = &mut self.line_buf_whole[self.nbytes_prompt..];
        while ibyte + nbytes_delete < self.ibyte_end && line_buf[ibyte + nbytes_delete] & 0b1100_0000 == 0b1000_0000 {
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
            if ibyte_char_attr < OutputT::CharAttr::NBYTES {
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
            while ibyte + OutputT::CharAttr::NBYTES < buf.len() && is_skippable(buf[ibyte + OutputT::CharAttr::NBYTES]) {
                ibyte += OutputT::CharAttr::NBYTES + 1;
            }
            ibyte
        };
        let ibyte_end = {
            let mut ibyte = buf.len();
            while ibyte >= OutputT::CharAttr::NBYTES + 1 && is_skippable(buf[ibyte - 1]) {
                ibyte -= OutputT::CharAttr::NBYTES + 1;
            }
            ibyte
        };
        (ibyte_start, ibyte_end)
    }
}


//-----------------------------------------------------------------------------
// Simple tasks
//-----------------------------------------------------------------------------
#[allow(dead_code)]
pub async fn simple_task_console<Writer, KeyReader>
    (writer: Writer, config: ConsoleConfig, key_reader: KeyReader, enable_break: bool)
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
    let editor = with_console::<CharAttr16ColorsDeco>(line_buf, history_buf, writer, config);
    simple_task_common(editor, key_reader, enable_break).await;
}

#[allow(dead_code)]
pub async fn simple_task_graphics<DrawTarget, ColorType, KeyReader, DelayNs, FnRefresh>
    (target: DrawTarget, config: GraphicsConfig<ColorType>, key_reader: KeyReader, enable_break: bool, delay: DelayNs, fn_refresh: FnRefresh)
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
        const BUFSIZE: usize = 1024;
        static STATIC_CELL: StaticCell<[u8; BUFSIZE]> = StaticCell::new();
        STATIC_CELL.init([0u8; BUFSIZE])
    };
    let editor = with_graphics::<CharAttrMonoColor, _, _, _>(
        line_buf, history_buf, target, rollback_buf, config, delay, fn_refresh);
    simple_task_common(editor, key_reader, enable_break).await;
}

pub async fn simple_task_common<OutputT, KeyReader>(mut editor: Editor<'_, OutputT>, mut reader: KeyReader, enable_break: bool)
where
    OutputT: Output,
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
