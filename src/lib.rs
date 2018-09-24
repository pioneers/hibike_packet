#[macro_use]
extern crate cpython;
#[macro_use]
extern crate lazy_static;
extern crate memchr;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::panic;
use std::collections::VecDeque;
use std::cell::RefCell;

use cpython::{Python, PyResult, PyObject, PyBytes, PythonObject, PyErr, ToPyObject, PyTuple};
use cpython::exc;

#[macro_use]
mod utils;
mod parsing;
use parsing::{initialize_parser_maps, parse_device_data};
use utils::objectify;


/// Change a Rust panic into a Python exception. Put this on all wrapper methods unless
/// you are absolutely certain it will not panic.
macro_rules! panic_to_except {
    ($py: ident, $result: expr) => {
        match panic::catch_unwind(panic::AssertUnwindSafe(|| $result)) {
            Err(e) => {
                if let Some(s) = e.downcast_ref::<&String>() {
                    Err(PyErr::new::<exc::RuntimeError, _>($py, format!("Rust code panicked: {}", s)))
                } else {
                    Err(PyErr::new::<exc::RuntimeError, _>($py, "Rust code panicked for an unknown reason"))
                }
            }
            Ok(r) => r,
        }
    };
}


/// Raw message data.
struct RawMessage {
    pub message_id: u8,
    pub payload: Vec<u8>,
}

/// Try to parse `bytes` into a packet.
fn parse_bytes_raw(bytes: &[u8]) -> Option<RawMessage> {
    let (cobs_frame, msg_size) = (bytes[0], bytes[1] as usize);
    if cobs_frame != 0 || bytes.len() < msg_size + 2 {
        return None;
    }

    let message = cobs_decode(&bytes[2..msg_size + 2]);
    let message_len = message.len();
    if message_len < 2 {
        return None;
    }
    let (message_id, payload_len) = (message[0], message[1] as usize);
    if message_len < 2 + payload_len + 1 {
        return None;
    }
    let payload = &message[2..2 + payload_len];
    let msg_checksum = message[2 + payload_len];
    if msg_checksum != checksum(&message[..message_len - 1]) {
        return None;
    }

    Some(RawMessage {
        message_id: message_id,
        payload: payload.into()
    })
}


py_class!(class RingBuffer |py| {
    data buffer: RefCell<VecDeque<u8>>;

    def __new__(_cls) -> PyResult<RingBuffer> {
        RingBuffer::create_instance(py, RefCell::new(VecDeque::new()))
    }

    def __len__(&self) -> PyResult<usize> {
        Ok(self.buffer(py).borrow().len())
    }

    def __repr__(&self) -> PyResult<String> {
        let buffer = self.buffer(py).borrow();
        let entries: Vec<String> = buffer.iter().map(|x| format!(r#"\x{:02x}"#, x)).collect();
        Ok(entries.join(""))
    }

    def find(&self, byte: u8) -> PyResult<Option<usize>> {
        let buffer = self.buffer(py).borrow();
        let (first_half, second_half) = buffer.as_slices();
        let first_loc = memchr::memchr(byte, first_half);
        
        Ok(first_loc.or_else(|| {
            memchr::memchr(byte, second_half).and_then(|loc| Some(loc + first_half.len()))
        }))
    }

    def count(&self, byte: u8) -> PyResult<u32> {
        let buffer = self.buffer(py).borrow();
        Ok(buffer.iter().map(|&x| (x == byte) as u32).sum())
    }

    def chop_front(&self, num: usize) -> PyResult<PyObject> {
        let mut buffer = self.buffer(py).borrow_mut();
        for _ in 0..num {
            buffer.pop_front();
        }
        Ok(py.None())
    }

    def extend(&self, data: PyBytes) -> PyResult<PyObject> {
        let cloned_data: Vec<u8> = data.data(py).into();
        let mut buffer = self.buffer(py).borrow_mut();
        buffer.append(&mut cloned_data.into());
        Ok(py.None())
    }

    def get_data(&self) -> PyResult<Vec<u8>> {
        let buffer = self.buffer(py).borrow();
        Ok(buffer.iter().cloned().collect())
    }
});

const DELIMITER: u8 = 0;
/// Search for packets in `buffer`, decoding them if found.
///
/// Returns:
/// - Python `None` if no packet was found
/// - Tuple of `(message_id, payload)` otherwise
fn process_buffer(gil: Python, buffer: RingBuffer) -> PyResult<PyObject> {
    let py_none = gil.None();
    let data = &buffer.get_data(gil)?;
    if let Some(curr_idx) = memchr::memchr(DELIMITER, data) {
        let chopped_data = &data[curr_idx..];
        match parse_bytes_raw(chopped_data) {
            Some(packet) => {
                // Chop off the packet data so we don't parse it again
                buffer.chop_front(gil, curr_idx + 1)?;
                let tuple = PyTuple::new(gil, &[objectify(gil, packet.message_id),
                                                objectify(gil, PyBytes::new(gil, &packet.payload))]);
                return Ok(objectify(gil, tuple));
            }
            None => {
                // Jump to the next packet, if there is one
                if let Some(next_idx) = memchr::memchr(DELIMITER, &chopped_data[1..]) {
                    buffer.chop_front(gil, curr_idx + next_idx + 1)?;
                }
            }
        }
    }

    return Ok(py_none);
}

/// COBS-decode `data`.
fn cobs_decode(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut index = 0usize;
    while index < data.len() {
        let block_size = (data[index] - 1) as usize;
        index += 1;
        if index + block_size > data.len() {
            return vec![];
        }
        output.extend_from_slice(&data[index..index + block_size]);
        index += block_size;
        if block_size + 1 < 255 && index < data.len() {
            output.push(0);
        }
    }
    output
}

/// Calculate a checksum for a message.
pub fn checksum(message: &[u8]) -> u8 {
    let mut sum = 0;
    for byte in message.iter() {
        sum ^= *byte;
    }
    sum
}

fn checksum_wrapper(gil: Python, message: PyBytes) -> PyResult<u8> {
    Ok(checksum(message.data(gil)))
}

py_module_initializer!(hibike_packet, inithibike_packet, PyInit_hibike_packet, |py, m| {
    m.add(py, "process_buffer", py_fn!(py, process_buffer(buffer: RingBuffer)))?;
    m.add(py, "checksum", py_fn!(py, checksum_wrapper(message: PyBytes)))?;
    m.add(py, "initialize_parser_maps", py_fn!(py, initialize_parser_maps(config_data: &str)))?;
    m.add(py, "parse_device_data", py_fn!(py, parse_device_data(payload: PyBytes, device_id: u16)))?;
    m.add_class::<RingBuffer>(py)?;
    Ok(())
});

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
