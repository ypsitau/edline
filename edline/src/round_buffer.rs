use embedded_io_async::Seek;
use unicode_width::UnicodeWidthStr;
use crate::utf8_accumulator::Utf8Accumulator;

//-----------------------------------------------------------------------------
// RoundBuffer
//-----------------------------------------------------------------------------
pub struct RoundBuffer<'a> {
    buf: &'a mut [u8],
    ibyte_top: usize,
    ibyte_bottom: usize,
    ibyte_write: usize,
    buf_full: bool,
    n_items: usize,
}

#[allow(dead_code)]
impl<'a> RoundBuffer<'a> {
    pub const IBYTE_NONE: usize = usize::MAX;

    pub fn new(buf: &'a mut [u8]) -> Self {
        buf.fill(0x00);
        Self { buf, ibyte_top: 0, ibyte_bottom: 0, ibyte_write: 0, buf_full: false, n_items: 0 }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }

    pub fn is_empty(&self) -> bool { self.n_items == 0 }

    pub fn buf(&self) -> &[u8] { self.buf }

    pub fn ibyte_top(&self) -> usize { self.ibyte_top }

    pub fn ibyte_bottom(&self) -> usize { self.ibyte_bottom }

    pub fn ibyte_write(&self) -> usize { self.ibyte_write }

    pub fn count(&self) -> usize { self.n_items }

    pub fn push(&mut self, src: &[u8]) -> Result<(), ()> {
        self.append_begin();
        self.append(src, true);
        Ok(())
    }

    pub fn append_begin(&mut self) {
        if self.n_items == 0 { self.ibyte_top = self.ibyte_write; }
        self.ibyte_bottom = self.ibyte_write;
    }

    fn append(&mut self, src: &[u8], set_terminator: bool) {
        assert!(src.len() + 1 <= self.capacity());
        let mut offset_write_end = self.ibyte_write + src.len();
        if set_terminator { offset_write_end += 1; }
        if offset_write_end <= self.buf.len() {
            if self.buf_full {
                while self.ibyte_write <= self.ibyte_top && self.ibyte_top < offset_write_end && self.n_items > 0 {
                    let mut idx = self.ibyte_top;
                    while self.buf[idx] != 0x00 {
                        idx += 1;
                        if idx >= self.buf.len() { idx = 0; }
                    }
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                    self.ibyte_top = idx;
                    self.n_items -= 1;
                }
            }
            self.buf[self.ibyte_write..self.ibyte_write + src.len()].copy_from_slice(src);
            self.ibyte_write = if offset_write_end == self.buf.len() { 0 } else { offset_write_end };
        } else {
            self.buf_full = true;
            let nbytes_copy = self.buf.len() - self.ibyte_write;
            let nbytes_rest = src.len() - nbytes_copy;
            offset_write_end = nbytes_rest;
            if set_terminator { offset_write_end += 1; }
            while (self.ibyte_top < offset_write_end || self.ibyte_write <= self.ibyte_top) && self.n_items > 0 {
                let mut idx = self.ibyte_top;
                while self.buf[idx] != 0x00 {
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                }
                idx += 1;
                if idx >= self.buf.len() { idx = 0; }
                self.ibyte_top = idx;
                self.n_items -= 1;
            }
            self.buf[self.ibyte_write..self.ibyte_write + nbytes_copy].copy_from_slice(&src[..nbytes_copy]);
            self.buf[0..nbytes_rest].copy_from_slice(&src[nbytes_copy..]);
            self.ibyte_write = offset_write_end;
        }
        if set_terminator {
            self.buf[offset_write_end - 1] = 0x00;
            self.n_items += 1;
        }
    }

    pub fn seek_item_next(&self, item_index: &mut RoundBufferItemIndex) -> bool {
        if item_index.ibyte == Self::IBYTE_NONE {
            if self.is_empty() { return false; }
            item_index.ibyte = self.ibyte_top;
        } else if item_index.ibyte == self.ibyte_bottom {
            return false;
        } else {
            item_index.ibyte = {
                let mut idx = item_index.ibyte;
                while self.buf[idx] != 0x00 {
                    idx = if idx + 1 < self.buf.len() { idx + 1 } else { 0 };
                    assert!(idx != item_index.ibyte, "Failed to find null terminator in buffer");
                }
                if idx + 1 < self.buf.len() { idx + 1 } else { 0 }
            };
        }
        true
    }

    pub fn seek_item_prev(&self, item_index: &mut RoundBufferItemIndex) -> bool {
        if item_index.ibyte == Self::IBYTE_NONE {
            if self.is_empty() { return false; }
            item_index.ibyte = self.ibyte_bottom;
        } else if item_index.ibyte == self.ibyte_top {
            return false;
        } else {
            item_index.ibyte = {
                let mut idx = item_index.ibyte;
                idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                let mut offset_start;
                loop {
                    offset_start = idx;
                    if offset_start == self.ibyte_top { break; }
                    idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                    if self.buf[idx] == 0x00 { break; }
                    assert!(idx != item_index.ibyte, "Failed to find null terminator in buffer");
                }
                offset_start
            }
        }
        true
    }

    pub fn get_item(&self, item_index: RoundBufferItemIndex) -> Option<(&[u8], &[u8])> {
        if item_index.ibyte == Self::IBYTE_NONE || self.is_empty() { return None; }
        let offset_end = {
            let mut ibyte = item_index.ibyte;
            while self.buf[ibyte] != 0x00 {
                ibyte = if ibyte + 1 < self.buf.len() { ibyte + 1 } else { 0 };
                assert!(ibyte != item_index.ibyte, "Failed to find null terminator in buffer");
            }
            ibyte
        };
        if offset_end == 0 {
            Some((&self.buf[item_index.ibyte..], &self.buf[0..0]))
        } else if offset_end < item_index.ibyte {
            Some((&self.buf[item_index.ibyte..], &self.buf[..offset_end]))
        } else {
            Some((&self.buf[item_index.ibyte..offset_end], &self.buf[0..0]))
        }
    }

    pub fn is_item_equal(&self, item_index: RoundBufferItemIndex, buf: &[u8]) -> bool {
        if let Some((first, second)) = self.get_item(item_index) {
            first.len() + second.len() == buf.len() &&
            &buf[..first.len()] == first && &buf[first.len()..] == second
        } else {
            false
        }
    }

    pub fn read_item(&self, item_index: RoundBufferItemIndex, buf: &mut [u8]) -> usize {
        if let Some((first, second)) = self.get_item(item_index) {
            let ibyte_end = first.len() + second.len();
            assert!(ibyte_end <= buf.len());
            buf[..first.len()].copy_from_slice(first);
            buf[first.len()..ibyte_end].copy_from_slice(second);
            ibyte_end
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundBufferItemIndex {
    pub ibyte: usize,
}

impl RoundBufferItemIndex {
    pub fn new() -> Self {
        Self { ibyte: RoundBuffer::IBYTE_NONE }
    }
}

pub struct WrappedRoundBuffer<'a> {
    round_buf: RoundBuffer<'a>,
    n_cols_wrap: usize,
    n_cols: usize,
    utf8_acc: Utf8Accumulator,
}

impl<'a> WrappedRoundBuffer<'a> {
    pub fn new(buf: &'a mut [u8], n_cols_wrap: usize) -> Self {
        WrappedRoundBuffer {
            round_buf: RoundBuffer::new(buf),
            n_cols_wrap,
            n_cols: 0,
            utf8_acc: Utf8Accumulator::new(),
        }
    }

    pub fn round_buf(&mut self) -> &RoundBuffer<'a> {
        &self.round_buf
    }

    pub fn push(&mut self, src: &[u8]) -> Result<(), ()> {
        let mut ibyte_start = 0;
        let mut ibyte_end = 0;
        while ibyte_end < src.len() {
            if src[ibyte_end] == b'\n' {
                if self.n_cols == 0 { self.round_buf.append_begin(); }
                self.round_buf.append(&src[ibyte_start..ibyte_end], true);
                ibyte_start = ibyte_end + 1;
                self.n_cols = 0;
            } else if self.utf8_acc.push(src[ibyte_end]) {
                if self.n_cols == 0 { self.round_buf.append_begin(); }
                let s = self.utf8_acc.get_result();
                let len_char = s.len();
                let n_cols_char = UnicodeWidthStr::width(s);
                self.utf8_acc.reset();
                if self.n_cols + n_cols_char < self.n_cols_wrap {
                    self.n_cols += n_cols_char;
                } else if self.n_cols + n_cols_char == self.n_cols_wrap {
                    self.round_buf.append(&src[ibyte_start..=ibyte_end], true);
                    ibyte_start = ibyte_end + 1;
                    self.n_cols = 0;
                } else { // self.n_cols + n_cols_char > self.n_cols_wrap
                    let ibyte_prev = ibyte_end + 1 - len_char;
                    self.round_buf.append(&src[ibyte_start..ibyte_prev], true);
                    ibyte_start = ibyte_prev;
                    self.round_buf.append_begin();
                    self.round_buf.append(&src[ibyte_start..=ibyte_end], false);
                    self.n_cols = n_cols_char;
                }
            }
            ibyte_end += 1;
        }
        if ibyte_start < src.len() {
            self.round_buf.append(&src[ibyte_start..], false);
        }
        Ok(())
    }

    pub fn flush(&mut self) {
        if self.n_cols > 0 {
            self.round_buf.append(&[], true);
            self.n_cols = 0;
        }
    }
}
