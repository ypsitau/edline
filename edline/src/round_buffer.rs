use unicode_width::UnicodeWidthStr;
use crate::utf8_accumulator::Utf8Accumulator;

//-----------------------------------------------------------------------------
// RoundBuffer
//-----------------------------------------------------------------------------
pub struct RoundBuffer<'a> {
    buf: &'a mut [u8],
    offset_top: usize,
    offset_bottom: usize,
    offset_write: usize,
    buf_full: bool,
    n_items: usize,
}

#[allow(dead_code)]
impl<'a> RoundBuffer<'a> {
    pub const OFFSET_NONE: usize = usize::MAX;

    pub fn new(buf: &'a mut [u8]) -> Self {
        buf.fill(0x00);
        Self { buf, offset_top: 0, offset_bottom: 0, offset_write: 0, buf_full: false, n_items: 0 }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }

    pub fn is_empty(&self) -> bool { self.n_items == 0 }

    pub fn buf(&self) -> &[u8] { self.buf }

    pub fn offset_top(&self) -> usize { self.offset_top }

    pub fn offset_bottom(&self) -> usize { self.offset_bottom }

    pub fn offset_write(&self) -> usize { self.offset_write }

    pub fn count(&self) -> usize { self.n_items }

    pub fn wrap_push(&mut self, src: &[u8], n_cols_wrap: usize) -> Result<(), ()> {
        let mut offset_start = 0;
        let mut offset_end = 0;
        let mut n_cols = 0;
        let mut utf8_acc = Utf8Accumulator::new();
        while offset_end < src.len() {
            if src[offset_end] == b'\n' {
                self.push(&src[offset_start..offset_end])?;
                offset_start = offset_end + 1;
            } else if utf8_acc.push(src[offset_end]) {
                let s = utf8_acc.get_result();
                let len_char = s.len();
                let n_cols_char = UnicodeWidthStr::width(s);
                utf8_acc.reset();
                if n_cols + n_cols_char < n_cols_wrap {
                    n_cols += n_cols_char;
                } else if n_cols + n_cols_char == n_cols_wrap {
                    self.push(&src[offset_start..=offset_end])?;
                    offset_start = offset_end + 1;
                    n_cols = 0;
                } else {
                    let offset_prev = offset_end + 1 - len_char;
                    self.push(&src[offset_start..offset_prev])?;
                    offset_start = offset_prev;
                    n_cols = n_cols_char;
                }
            }
            offset_end += 1;
        }
        if offset_start < src.len() {
            self.push(&src[offset_start..])?;
        }
        Ok(())
    }

    pub fn push(&mut self, src: &[u8]) -> Result<(), ()> {
        if src.len() + 1 > self.capacity() { return Err(()); }
        if self.n_items == 0 { self.offset_top = self.offset_write; }
        self.offset_bottom = self.offset_write;
        self.append(src, true);
        self.n_items += 1;
        Ok(())
    }

    fn append(&mut self, src: &[u8], set_terminator: bool) {
        let mut offset_write_end = self.offset_write + src.len();
        if set_terminator { offset_write_end += 1; }
        if offset_write_end <= self.buf.len() {
            if self.buf_full {
                while self.offset_write <= self.offset_top && self.offset_top < offset_write_end && self.n_items > 0 {
                    let mut idx = self.offset_top;
                    while self.buf[idx] != 0x00 {
                        idx += 1;
                        if idx >= self.buf.len() { idx = 0; }
                    }
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                    self.offset_top = idx;
                    self.n_items -= 1;
                }
            }
            self.buf[self.offset_write..self.offset_write + src.len()].copy_from_slice(src);
            self.offset_write = if offset_write_end == self.buf.len() { 0 } else { offset_write_end };
        } else {
            self.buf_full = true;
            let nbytes_copy = self.buf.len() - self.offset_write;
            let nbytes_rest = src.len() - nbytes_copy;
            offset_write_end = nbytes_rest;
            if set_terminator { offset_write_end += 1; }
            while (self.offset_top < offset_write_end || self.offset_write <= self.offset_top) && self.n_items > 0 {
                let mut idx = self.offset_top;
                while self.buf[idx] != 0x00 {
                    idx += 1;
                    if idx >= self.buf.len() { idx = 0; }
                }
                idx += 1;
                if idx >= self.buf.len() { idx = 0; }
                self.offset_top = idx;
                self.n_items -= 1;
            }
            self.buf[self.offset_write..self.offset_write + nbytes_copy].copy_from_slice(&src[..nbytes_copy]);
            self.buf[0..nbytes_rest].copy_from_slice(&src[nbytes_copy..]);
            self.offset_write = offset_write_end;
        }
        if set_terminator { self.buf[offset_write_end - 1] = 0x00; }
    }

    pub fn seek_item_next(&self, offset_seek: &mut usize) -> bool {
        if *offset_seek == Self::OFFSET_NONE {
            if self.is_empty() { return false; }
            *offset_seek = self.offset_top;
        } else if *offset_seek == self.offset_bottom {
            return false;
        } else {
            *offset_seek = {
                let mut idx = *offset_seek;
                while self.buf[idx] != 0x00 {
                    idx = if idx + 1 < self.buf.len() { idx + 1 } else { 0 };
                    assert!(idx != *offset_seek, "Failed to find null terminator in buffer");
                }
                if idx + 1 < self.buf.len() { idx + 1 } else { 0 }
            };
        }
        true
    }

    pub fn seek_item_prev(&self, offset_seek: &mut usize) -> bool {
        if *offset_seek == Self::OFFSET_NONE {
            if self.is_empty() { return false; }
            *offset_seek = self.offset_bottom;
        } else if *offset_seek == self.offset_top {
            return false;
        } else {
            *offset_seek = {
                let mut idx = *offset_seek;
                idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                let mut offset_start;
                loop {
                    offset_start = idx;
                    if offset_start == self.offset_top { break; }
                    idx = if idx == 0 { self.buf.len() - 1 } else { idx - 1 };
                    if self.buf[idx] == 0x00 { break; }
                    assert!(idx != *offset_seek, "Failed to find null terminator in buffer");
                }
                offset_start
            }
        }
        true
    }

    pub fn get_item(&self, offset_item: usize) -> Option<(&[u8], &[u8])> {
        if offset_item == Self::OFFSET_NONE || self.is_empty() { return None; }
        let offset_end = {
            let mut idx = offset_item;
            while self.buf[idx] != 0x00 {
                idx = if idx + 1 < self.buf.len() { idx + 1 } else { 0 };
                assert!(idx != offset_item, "Failed to find null terminator in buffer");
            }
            idx
        };
        if offset_end == 0 {
            Some((&self.buf[offset_item..], &self.buf[0..0]))
        } else if offset_end < offset_item {
            Some((&self.buf[offset_item..], &self.buf[..offset_end]))
        } else {
            Some((&self.buf[offset_item..offset_end], &self.buf[0..0]))
        }
    }

    pub fn is_item_equal(&self, offset_item: usize, buf: &[u8]) -> bool {
        if let Some((first, second)) = self.get_item(offset_item) {
            first.len() + second.len() == buf.len() &&
            &buf[..first.len()] == first && &buf[first.len()..] == second
        } else {
            false
        }
    }

    pub fn read_item(&self, offset_item: usize, buf: &mut [u8]) -> usize {
        if let Some((first, second)) = self.get_item(offset_item) {
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
