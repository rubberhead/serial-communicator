#![allow(dead_code)]

extern crate serialport; 

use std::error::Error; 
use std::io::{self, BufRead, BufReader}; 
use serialport::SerialPort;  

/// Tries to read a raw QWORD from the given `port`.
/// 
/// This function gives no concern to endianness. 
pub fn read_qword_raw(port: &mut dyn SerialPort) -> Result<u64, io::Error> {
    let mut buf = [0 as u8; 8]; 
    port.read_exact(&mut buf)?;
    let qword_ptr: *const u64 = (&buf as *const u8).cast(); 
    unsafe {
        return Ok(*qword_ptr); 
    }
}

/// Tries to read a raw QWORD from the given `port`, 
/// then converts it to the opposite endian.
/// 
/// Useful for, say, reading x86-based numeric values on an ARM machine. 
pub fn read_qword_flipped_endian(port: &mut dyn SerialPort) -> Result<u64, io::Error> {
    let mut buf = [0 as u8; 8]; 
    port.read_exact(&mut buf)?; 
    buf.reverse(); 
    unsafe {
        return Ok(*(&buf as *const u8).cast()); 
    }
}

/// Tries to write a raw QWORD to the given `port`.
/// 
/// This function gives no concern to endianness. 
pub fn write_qword_raw(port: &mut dyn SerialPort, ref val: u64) -> Result<(), io::Error> {
    let buf_ptr: *const [u8; 8] = (val as *const u64).cast(); 
    unsafe {
        return port.write_all(&*buf_ptr); 
    }
}

/// Tries to write a QWORD with flipped endian to the given `port`.
/// 
/// Useful for, say, writing x86-based numerics to ARM machines.
pub fn write_qword_flipped_endian(port: &mut dyn SerialPort, ref mut val: u64) -> Result<(), io::Error> {
    let buf_ptr: *mut [u8; 8] = (val as *mut u64).cast(); 
    unsafe {
        let buf: &mut [u8; 8] = &mut *buf_ptr; 
        buf.reverse();
        return port.write_all(buf); 
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
    let mut buf = [0 as u8; 4]; 
    port.read_exact(&mut buf)?;
    let dword_ptr: *const u32 = (&buf as *const u8).cast();
    unsafe {
        return Ok(*dword_ptr); 
    }
}

/// Tries to read a raw DWORD from the given `port`, 
/// then converts it to the opposite endian. 
/// 
/// Useful for, say, reading x86-based numeric values on an ARM machine. 
pub fn read_dword_flipped_endian(port: &mut dyn SerialPort) -> Result<u32, io::Error> {
    let mut buf = [0 as u8; 4]; 
    port.read_exact(&mut buf)?; 
    buf.reverse(); 
    unsafe {
        return Ok(*(&buf as *const u8).cast()); 
    }
}

/// Tries to write a raw DWORD to the given `port`. 
/// 
/// This function gives no concern to endianness. 
pub fn write_dword_raw(port: &mut dyn SerialPort, ref val: u32) -> Result<(), io::Error> {
    let buf_ptr: *const [u8; 4] = (val as *const u32).cast(); 
    unsafe {
        return port.write_all(&*buf_ptr); 
    }
}

/// Tries to write a DWORD with flipped endian to the given `port`. 
/// 
/// Useful for, say, writing x86-based numerics to ARM machines. 
pub fn write_dword_flipped_endian(port: &mut dyn SerialPort, ref mut val: u32) -> Result<(), io::Error> {
    let buf_ptr: *mut [u8; 4] = (val as *mut u32).cast(); 
    unsafe {
        let buf: &mut [u8; 4] = &mut *buf_ptr;
        buf.reverse(); 
        return port.write_all(buf); 
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
    return Ok(String::from_utf8(buf)?); 
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
    let endbyte_ptr: *const [u8; 1] = (&endbyte as *const u8).cast(); 
    port.write_all(str_to_write.as_bytes())?;
    unsafe {
        return port.write_all(&*endbyte_ptr); 
    }
}