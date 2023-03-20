#![allow(dead_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::cast_possible_wrap)]

extern crate serialport;

use std::error::Error;
use std::fmt::Display;
use std::io::{self, BufRead, BufReader};
use serialport::SerialPort; 

/// Tries to read a raw QWORD from the given `port`.
///
/// This function gives no concern to endianness.
pub fn read_qword_raw(port: &mut dyn SerialPort) -> Result<u64, io::Error> {
    let mut buf = [0_u8; 8];
    port.read_exact(&mut buf)?;
    let qword_ptr: *const u64 = std::ptr::addr_of!(buf).cast();
    unsafe {
        Ok(*qword_ptr)
    }
}

/// Tries to read a raw QWORD from the given `port`,
/// then converts it to the opposite endian.
///
/// Useful for, say, reading x86-based numeric values on an ARM machine.
pub fn read_qword_flipped_endian(port: &mut dyn SerialPort) -> Result<u64, io::Error> {
    let mut buf = [0_u8; 8];
    port.read_exact(&mut buf)?;
    buf.reverse();
    unsafe {
        Ok(*std::ptr::addr_of!(buf).cast())
    }
}

/// Tries to write a raw QWORD to the given `port`.
///
/// This function gives no concern to endianness.
pub fn write_qword_raw(port: &mut dyn SerialPort, val: u64) -> Result<(), io::Error> {
    let buf_ptr: *const [u8; 8] = (val as *const u64).cast();
    unsafe {
        port.write_all(&*buf_ptr)
    }
}

/// Tries to write a QWORD with flipped endian to the given `port`.
///
/// Useful for, say, writing x86-based numerics to ARM machines.
pub fn write_qword_flipped_endian(port: &mut dyn SerialPort, val: u64) -> Result<(), io::Error> {
    let buf_ptr: *mut [u8; 8] = (val as *mut u64).cast();
    unsafe {
        let buf: &mut [u8; 8] = &mut *buf_ptr;
        buf.reverse();
        port.write_all(buf)
    }
}

/// Tries to read a raw QWORD from the given `port` and converts it into `i64`.
///
/// This function gives no concern to endianness.
pub fn read_i64_raw(port: &mut dyn SerialPort) -> Result<i64, io::Error> {
    Ok(read_qword_raw(port)? as i64)
}

/// Tries to read a raw DWORD from the given `port`.
///
/// This function gives no concern to endianness.
pub fn read_dword_raw(port: &mut dyn SerialPort) -> Result<u32, io::Error> {
    let mut buf = [0_u8; 4];
    port.read_exact(&mut buf)?;
    let dword_ptr: *const u32 = std::ptr::addr_of!(buf).cast();
    unsafe {
        Ok(*dword_ptr)
    }
}

/// Tries to read a raw DWORD from the given `port`,
/// then converts it to the opposite endian.
///
/// Useful for, say, reading x86-based numeric values on an ARM machine.
pub fn read_dword_flipped_endian(port: &mut dyn SerialPort) -> Result<u32, io::Error> {
    let mut buf = [0_u8; 4];
    port.read_exact(&mut buf)?;
    buf.reverse();
    unsafe {
        Ok(*std::ptr::addr_of!(buf).cast())
    }
}

/// Tries to write a raw DWORD to the given `port`.
///
/// This function gives no concern to endianness.
pub fn write_dword_raw(port: &mut dyn SerialPort, val: u32) -> Result<(), io::Error> {
    let buf_ptr: *const [u8; 4] = (val as *const u32).cast();
    unsafe {
        port.write_all(&*buf_ptr)
    }
}

/// Tries to write a DWORD with flipped endian to the given `port`.
///
/// Useful for, say, writing x86-based numerics to ARM machines.
pub fn write_dword_flipped_endian(port: &mut dyn SerialPort, val: u32) -> Result<(), io::Error> {
    let buf_ptr: *mut [u8; 4] = (val as *mut u32).cast();
    unsafe {
        let buf: &mut [u8; 4] = &mut *buf_ptr;
        buf.reverse();
        port.write_all(buf)
    }
}

/// Tries to read a raw DWORD from the given `port` and converts it into `i64`.
///
/// This function gives no concern to endianness.
pub fn read_i32_raw(port: &mut dyn SerialPort) -> Result<i32, io::Error> {
    Ok(read_dword_raw(port)? as i32)
}

/// Tries to read a String from the given `port`.
///
/// ## Ok
/// Owned `String` containing the sent text until and including `endbyte`. I don't make the rules.
///
/// ## Err
/// - `io::Error` if cannot read from `port`.
/// - `alloc::string::FromUtf8Error` if cannot parse `u8` buffer to `String`.
pub fn read_string_until_byte(port: &mut dyn SerialPort, endbyte: u8) -> Result<String, Box<dyn Error>> {
    let mut br = BufReader::new(port); 
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    br.read_until(endbyte, &mut buf)?; 
    Ok(String::from_utf8(buf)?)
}

/// Tries to write a string slice into the given `port`.
pub fn write_str_raw(port: &mut dyn SerialPort, str_to_write: &str) -> Result<(), io::Error> {
    port.write_all(str_to_write.as_bytes())
}

/// Tries to write a string slice into the given `port`, appending `endbyte` at behind.
pub fn write_str_ends_with(
    port: &mut dyn SerialPort,
    str_to_write: &str,
    endbyte: u8
) -> Result<(), io::Error> {
    let endbyte_ptr: *const [u8; 1] = std::ptr::addr_of!(endbyte).cast();
    port.write_all(str_to_write.as_bytes())?;
    unsafe {
        port.write_all(&*endbyte_ptr)
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum Action {
    Read, 
    Write(String)
}

#[derive(Debug)]
pub enum ActionConversionError {
    UndefinedOpSequence(String), 
    EmptyOpSequence(String), 
    MalformedOpSequence(String), 
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Read => 
                write!(f, "READ"), 
            Action::Write(s) => 
                write!(f, "WRITE {s}"), 
        }
    }
}

impl TryFrom<&str> for Action {
    type Error = ActionConversionError;

    fn try_from(action: &str) -> Result<Self, Self::Error> {
        const _FN_NAME: &str = "[Action as TryFrom::try_from]"; 

        let mut split = action.split_ascii_whitespace(); 
        let op: &str; 
        match split.next() {
            Some("READ") => 
                return Ok(Action::Read), 
            Some(s) => 
                op = s, 
            None => 
                return Err(ActionConversionError::EmptyOpSequence(
                    format!("{_FN_NAME} Empty sequence as input")
                )), 
        }

        // [TODO] Assumming 1 argument...
        let mut arg_buff: String = String::with_capacity(action.len()); 
        while let Some(s) = split.next() {
            arg_buff.push_str(s); 
            arg_buff.push_str(" "); 
        }
        
        match op {
            "WRITE" => 
                return Ok(Action::Write(arg_buff)), 
            _ => 
                return Err(ActionConversionError::UndefinedOpSequence(
                    format!("{_FN_NAME} Undefined operation {op} from sequence {action}")
                )), 
        }
    }
}