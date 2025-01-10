#![no_main]
#![allow(dead_code)]

use breakwater_parser::Parser;
use serde::ser::SerializeSeq;
use sonic_rs::Serialize;
use std::cell::UnsafeCell;

#[derive(Debug, Clone, Copy, Serialize)]
enum FuzzBufferEvent {
    Set { x: usize, y: usize, color: u32 },
    SelMulti { start: usize, len: usize },
    Get { x: usize, y: usize },
}

#[derive(Default, Debug)]
struct FuzzBuffer {
    log: UnsafeCell<Vec<FuzzBufferEvent>>,
}

impl Serialize for FuzzBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let log = unsafe { &*self.log.get() };

        let mut seq = serializer.serialize_seq(Some(log.len()))?;
        for e in log {
            seq.serialize_element(e)?
        }
        seq.end()
    }
}

impl breakwater_parser::FrameBuffer for FuzzBuffer {
    fn get_width(&self) -> usize {
        1920
    }

    fn get_height(&self) -> usize {
        1080
    }

    unsafe fn get_unchecked(&self, x: usize, y: usize) -> u32 {
        unsafe { &mut *self.log.get() }.push(FuzzBufferEvent::Get { x, y });
        0
    }

    fn set(&self, x: usize, y: usize, rgba: u32) {
        unsafe { &mut *self.log.get() }.push(FuzzBufferEvent::Set { x, y, color: rgba });
    }

    fn set_multi_from_start_index(&self, starting_index: usize, pixels: &[u8]) -> usize {
        unsafe { &mut *self.log.get() }.push(FuzzBufferEvent::SelMulti {
            start: starting_index,
            len: pixels.len(),
        });
        starting_index + pixels.len()
    }

    fn as_bytes(&self) -> &[u8] {
        todo!()
    }

    fn as_pixels(&self) -> &[u32] {
        todo!()
    }
}

#[derive(Debug, Serialize)]
struct FuzzRun<'a> {
    input: std::borrow::Cow<'a, str>,
    response: std::borrow::Cow<'a, str>,
    framebuffer_ops: std::sync::Arc<FuzzBuffer>,
}

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    #[allow(clippy::arc_with_non_send_sync)]
    let fb = std::sync::Arc::new(FuzzBuffer::default());
    let mut parser = breakwater_parser::OriginalParser::new(fb.clone());
    let mut resp_buf = vec![];
    parser.parse(data, &mut resp_buf);

    if std::env::var("REPORT").is_ok() {
        let run = FuzzRun {
            input: String::from_utf8_lossy(data),
            response: String::from_utf8_lossy(&resp_buf),
            framebuffer_ops: fb,
        };
        print!("{}", sonic_rs::to_string_pretty(&run).expect("json"));
    }
});
