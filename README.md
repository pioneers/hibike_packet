# Hibike Packet Utilities

A C extension for Hibike to increase its speed.

## API

### `RingBuffer`
`RingBuffer` is a deque that stores bytes.
- `__init__()`: create a `RingBuffer`
- `chop_front(n)`: Remove `n` bytes from the front of the buffer
- `extend(bytes)`: place `bytes` at the back of the buffer

### `process_buffer()`
- `process_buffer(buf: RingBuffer) -> Union[(int, bytes), None]`
  + Try to parse a packet from `buf`, consuming its bytes in the process
  + Returns `(msg_id, payload)` if successful, otherwise `None`

## Building a Wheel
Run
```bash
./build.sh
```

This will produce a wheel in `build/dist`, which you can then install with `pip`.
