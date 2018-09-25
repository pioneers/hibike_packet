# Hibike Packet Utilities

A C extension for Hibike to increase its speed.

## API
See [the `typing` module](https://docs.python.org/3/library/typing.html) for definitions of `Union` and `List`.

### `RingBuffer`
`RingBuffer` is a deque that stores bytes.
- `__init__()`: create a `RingBuffer`
- `chop_front(n)`: Remove `n` bytes from the front of the buffer
- `extend(bytes)`: place `bytes` at the back of the buffer

### `process_buffer()`
- `process_buffer(buf: RingBuffer) -> Union[(int, bytes), None]`
  + Try to parse a packet from `buf`, consuming its bytes in the process
  + Returns `(msg_id, payload)` if successful, otherwise `None`

### `initialize_parser_maps()`
- `initialize_parser_maps(json: str)`
  + Initialize parsing state for `parse_device_data`
  + `json` should be the contents of `hibikeDevices.json`
  + This function should be called before `parse_device_data`

### `parse_device_data()`
- `parse_device_data(payload: bytes, device_id: int) -> List[(str, Union[float, bool, int])]`
  + Attempt to parse `payload` into device data
  + Raises `AssertionError` upon invalid device ID or bad packet format


## Building a Wheel
Run
```bash
./build.sh
```

## Installing
Assuming a successful build, there should be a `.whl` file located in
`build/dist`. Just do
```bash
pip install build/dist/NAME_OF_FILE
```

## Rust Documentation
Building documentation for the source code is simple:
```bash
cargo doc
```
The built documentation is located in `doc/hibike_packet`.
