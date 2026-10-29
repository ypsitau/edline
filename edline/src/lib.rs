//#![no_std]
use embedded_io_async as io_async;

pub trait Output {
    type Error;
    async fn write(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    async fn save_cursor_position(&mut self) -> Result<(), Self::Error>;
    async fn restore_cursor_position(&mut self) -> Result<(), Self::Error>;
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
    async fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.writer.write_all(data).await?;
        self.writer.flush().await?;
        Ok(())
    }
    async fn save_cursor_position(&mut self) -> Result<(), Self::Error> {
        self.writer.write_all(b"\x1b7").await?;
        self.writer.flush().await?;
        Ok(())
    }
    async fn restore_cursor_position(&mut self) -> Result<(), Self::Error> {
        self.writer.write_all(b"\x1b8").await?;
        self.writer.flush().await?;
        Ok(())
    }
}
pub struct Editor<'a, Output> {
    output: Output,
    line_buf: &'a mut [u8],
    history_buf: &'a mut [u8],
    icur: usize,
    icur_end: usize,
}

impl<'a, OutputT: Output> Editor<'a, OutputT> {
    pub fn new(output: OutputT, line_buf: &'a mut [u8], history_buf: &'a mut [u8]) -> Self {
        Self { output, line_buf, history_buf, icur: 0, icur_end: 0 }
    }
    pub async fn start(&mut self) {
        self.output.save_cursor_position().await;
    }
    pub async fn process_key(&mut self, key: serkey::Key) {
        //println!("Received key: {:?}", key);
        match key {
            serkey::Key::CookedChar(c) => {
                if self.icur < self.line_buf.len() {
                    self.line_buf[self.icur] = c as u8;
                    self.icur += 1;
                    if self.icur_end < self.icur {
                        self.icur_end = self.icur;
                    }
                }
                self.output.restore_cursor_position().await;
                self.output.write(&self.line_buf[..self.icur]).await;
            },
            //serkey::Key::Backspace => { self.line_buf.pop(); },
            _ => {}
        }
        // Handle the key input here
    }
}
