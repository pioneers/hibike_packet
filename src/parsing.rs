extern crate serde_json;
extern crate byteorder;

use ::utils::{value_error, objectify};

use std::collections::HashMap;
use std::sync::RwLock;
use std::io::Cursor;
use std::io;

use cpython::{Python, PyResult, PyObject, PyBytes, PyList, PyErr, ToPyObject, exc};

use self::byteorder::{LittleEndian, ReadBytesExt};

type DeviceId = u16;
type ParamMap = HashMap<String, Parameter>;

lazy_static! {
    static ref PARAM_MAP: RwLock<HashMap<DeviceId, ParamMap>> = {
        RwLock::new(HashMap::new())
    };

    static ref DEVICE_MAP: RwLock<HashMap<DeviceId, Device>> = {
        RwLock::new(HashMap::new())
    };

    static ref MESSAGE_TYPES: HashMap<&'static str, u8> = {
        let mut m = HashMap::new();
        m.insert("Ping", 0x10);
        m.insert("SubscriptionRequest", 0x11);
        m.insert("SubscriptionResponse", 0x12);
        m.insert("DeviceRead", 0x13);
        m.insert("DeviceWrite", 0x14);
        m.insert("Disable", 0x16);
        m.insert("HeartBeatRequest", 0x17);
        m.insert("HeartBeatResponse", 0x18);
        m.insert("Error", 0xFF);
        m
    };
    static ref ERROR_CODES: HashMap<&'static str, u8> = {
        let mut c = HashMap::new();
        c.insert("UnexpectedDelimiter", 0xFD);
        c.insert("ChecksumError", 0xFE);
        c.insert("GenericError", 0xFF);
        c
    };
}

#[derive(Clone, Deserialize)]
pub struct Device {
    pub id: u16,
    pub name: String,
    pub params: Vec<Parameter>,
}

#[derive(Copy, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    Bool,
    #[serde(rename = "uint8_t")]
    Uint8,
    #[serde(rename = "int8_t")]
    Int8,
    #[serde(rename = "uint16_t")]
    Uint16,
    #[serde(rename = "int16_t")]
    Int16,
    #[serde(rename = "uint32_t")]
    Uint32,
    #[serde(rename = "int32_t")]
    Int32,
    #[serde(rename = "uint64_t")]
    Uint64,
    #[serde(rename = "int64_t")]
    Int64,
    Float,
    Double
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub number: u8,
    #[serde(rename = "type")]
    pub kind: ParamType,
    pub read: bool,
    pub write: bool,
}

/// Initialize device parameters based on `config_data`, a JSON string.
pub fn initialize_parser_maps(gil: Python, config_data: &str) -> PyResult<PyObject> {
    let mut parsed_map: HashMap<DeviceId, Device> = match serde_json::from_str(config_data) {
        Ok(map) => map,
        Err(e) => {
            return Err(value_error(gil, format!("could not parse device parameters: {}", e)));
        }
    };
    let mut device_map = DEVICE_MAP.write().expect("Device map lock was poisoned");
    let mut param_map = PARAM_MAP.write().expect("Param map lock was poisoned");
    parsed_map.clone().into_iter().for_each(|(k, v)| { device_map.insert(k, v); });

    for (device_id, device) in parsed_map.drain() {
        let mut params = HashMap::new();
        for param in device.params {
            params.insert(param.name.clone(), param);
        }
        param_map.insert(device_id, params);
    }

    Ok(gil.None())
}

/// Decode `bitmask` into human-readable names.
fn decode_params(device_id: u16, bitmask: u16) -> Vec<String> {
    let device_map = DEVICE_MAP.read().unwrap();
    let device = &device_map[&device_id];
    let mut names: Vec<String> = Vec::with_capacity(16);
    for i in 0..16 {
        if i >= device.params.len() {
            break;
        }
        if bitmask & (1 << i) != 0 {
            let name: String = device.params[i].name.clone();
            names.push(name);
        }
    }

    names
}

fn try_read<T>(gil: Python, maybe_param: io::Result<T>) -> PyResult<PyObject> where T: ToPyObject {
    py_assert!(gil, maybe_param.is_ok(), "packet is missing a parameter");
    Ok(objectify(gil, maybe_param.unwrap()))
}

pub fn parse_device_data(gil: Python, payload: PyBytes, device_id: u16) -> PyResult<PyList> {
    let device_map = DEVICE_MAP.read().unwrap();
    py_assert!(gil, device_map.contains_key(&device_id), format!("invalid device_id: {}", device_id));
    let raw_bytes = payload.data(gil);
    py_assert!(gil, raw_bytes.len() >= 2, "Packet payload is too short");

    let mut cursor = Cursor::new(raw_bytes);
    let bitmask = cursor.read_u16::<LittleEndian>().unwrap();
    let names: Vec<String> = decode_params(device_id, bitmask);

    let mut values = Vec::with_capacity(16);
    let param_map = &PARAM_MAP.read().unwrap()[&device_id];
    for name in &names {
        let value = match &param_map[name].kind {
            ParamType::Uint8 => try_read(gil, cursor.read_u8())?,
            ParamType::Uint16 => try_read(gil, cursor.read_u16::<LittleEndian>())?,
            ParamType::Uint32 => try_read(gil, cursor.read_u32::<LittleEndian>())?,
            ParamType::Uint64 => try_read(gil, cursor.read_u64::<LittleEndian>())?,
            ParamType::Int8 => try_read(gil, cursor.read_i8())?,
            ParamType::Int16 => try_read(gil, cursor.read_i16::<LittleEndian>())?,
            ParamType::Int32 => try_read(gil, cursor.read_i32::<LittleEndian>())?,
            ParamType::Int64 => try_read(gil, cursor.read_i64::<LittleEndian>())?,
            ParamType::Float => try_read(gil, cursor.read_f32::<LittleEndian>())?,
            ParamType::Double => try_read(gil, cursor.read_f64::<LittleEndian>())?,
            ParamType::Bool => {
                let maybe_bool = cursor.read_u8()
                                       .and_then(|u| Ok(u != 0));
                py_assert!(gil, maybe_bool.is_ok(), "packet is missing a parameter");
                objectify(gil, maybe_bool.unwrap())
            }
        };
        values.push(value);
    }
    Ok(PyList::new(gil, &names.into_iter()
                              .zip(values)
                              .map(|tup| objectify(gil, tup))
                              .collect::<Vec<_>>()))
}
