#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate byteorder;
extern crate bytes;

use byteorder::*;
use bytes::BufMut;
use std::io::{self, Write};
use std::mem;

pub const CONTROL_ACCEPT: u32 = 0x01;
pub const CONTROL_START: u32 = 0x02;
pub const CONTROL_STOP: u32 = 0x03;
pub const CONTROL_READY: u32 = 0x04;
pub const CONTROL_FINISH: u32 = 0x05;
pub const CONTROL_FIELD_CONTENT_TYPE: u32 = 0x01;

#[derive(Clone, Debug)]
pub struct EncoderWriter<W: Write> {
    writer: Option<W>,
    content_type: Option<String>,
    partial: bool,
    started: bool,
}

impl<W: Write> Write for EncoderWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.started {
            self.started = true;
            try!(self.write_control_start());
        }
        self.write_frame(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.as_mut().unwrap().flush()
    }
}

impl<W: Write> EncoderWriter<W> {
    pub fn new(writer: W, content_type: Option<String>) -> EncoderWriter<W> {
        EncoderWriter {
            writer: Some(writer),
            content_type: content_type,
            partial: false,
            started: false,
        }
    }

    pub fn finish(mut self) -> io::Result<W> {
        if self.started {
            try!(self.write_control_stop());
            self.started = false;
        }
        try!(self.write_control_stop());
        Ok(self.writer.take().unwrap())
    }

    pub fn reset(&mut self, writer: W) -> io::Result<W> {
        if self.started {
            try!(self.write_control_stop());
            self.started = false;
        }
        Ok(mem::replace(&mut self.writer, Some(writer)).unwrap())
    }

    pub fn into_inner(mut self) -> W {
        self.writer.take().unwrap()
    }

    fn write_control_start(&mut self) -> io::Result<()> {
        let mut total_len = 0; // Escape
        total_len += 4;        // Frame length
        total_len += 4;        // Control type length
        total_len += 4;        // Control type
        if let Some(ref content_type) = self.content_type {
            total_len += 4;    // CONTROL_FIELD_CONTENT_TYPE
            total_len += 4;    // Length of content type string
            total_len += content_type.as_bytes().len();
        }
        let mut buf = Vec::with_capacity(total_len);
        buf.put_u32::<BigEndian>(0);                        // Escape
        buf.put_u32::<BigEndian>(total_len as u32 - 2 * 4); // Frame length
        buf.put_u32::<BigEndian>(CONTROL_START);            // Control type
        if let Some(ref content_type) = self.content_type {
            buf.put_u32::<BigEndian>(CONTROL_FIELD_CONTENT_TYPE);
            buf.put_u32::<BigEndian>(content_type.as_bytes().len() as u32);
            let _ = buf.write(content_type.as_bytes());
        }
        try!(self.writer.as_mut().unwrap().write_all(&buf));
        Ok(())
    }

    fn write_control_stop(&mut self) -> io::Result<()> {
        let total_len = 3 * 4;
        let mut buf = Vec::with_capacity(total_len);
        buf.put_u32::<BigEndian>(0);                        // Escape
        buf.put_u32::<BigEndian>(total_len as u32 - 2 * 4); // Frame length
        buf.put_u32::<BigEndian>(CONTROL_STOP);             // Control type
        try!(self.writer.as_mut().unwrap().write_all(&buf));
        Ok(())
    }

    fn write_frame(&mut self, frame: &[u8]) -> io::Result<usize> {
        let len = frame.len();
        let len_u32 = [(len >> 24) as u8, (len >> 16) as u8, (len >> 8) as u8, len as u8];
        if !self.partial {
            try!(self.writer.as_mut().unwrap().write_all(&len_u32));
        }
        let wlen = try!(self.writer.as_mut().unwrap().write(frame));
        if wlen < len {
            self.partial = true;
        }
        Ok(wlen)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_encoding() {
        use ::EncoderWriter;
        use std::io::prelude::*;
        use std::io::BufWriter;

        let mut enc = EncoderWriter::new(BufWriter::new(Vec::new()),
                                         Some("test-content-type".to_owned()));
        enc.write(b"test-content").unwrap();
        let enc = enc.finish().unwrap();
        let out = enc.into_inner().unwrap();
        let expected: [u8; 77] = [0, 0, 0, 0, 0, 0, 0, 29, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 17,
                                  116, 101, 115, 116, 45, 99, 111, 110, 116, 101, 110, 116, 45,
                                  116, 121, 112, 101, 12, 0, 0, 0, 116, 101, 115, 116, 45, 99,
                                  111, 110, 116, 101, 110, 116, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0,
                                  3, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 3];
        assert_eq!(out, &expected[..]);
    }
}
