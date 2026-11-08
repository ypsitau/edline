# edline

A line editor for embedded systems.

## Features

- Line editing capabilities for embedded systems
- History recall (previous and next)
- UTF-8 and VT100 terminal control sequence handling
- Asynchronous operation compatible with embedded-io-async
- Minimal dependencies for lightweight usage

## Example

```rust
use core::fmt::Write as _;
use embedded_io_async as io_async;
use static_cell::StaticCell;
use heapless::String;

pub async fn simple_task_stream(writer: impl io_async::Write, mut reader: impl io_async::Read) {
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
    let mut sbuf_result: heapless::String<128> = heapless::String::new();
    let mut editor = edline::Editor::with_serial_output(line_buf, history_buf, writer);
    let mut serkey_reader = serkey::Reader::new();
    let prompt = "> ";
    editor.print(prompt).await;
    loop {
        if let Ok(key) = serkey_reader.read_key(&mut reader).await {
            match editor.process_key(key).await {
                edline::Event::Entered => {
                    editor.read_result_str(&mut sbuf_result).ok();
                    editor.print("Result: ").await;
                    editor.print(&sbuf_result).await;
                    editor.print("\r\n").await;
                    editor.print(prompt).await;
                }
                _ => {}
            }
        }
    }
}
```

## License

This project is licensed under the MIT OR Apache-2.0 license.
