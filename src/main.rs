#![cfg(unix)]

extern crate serial_communicator; 

use std::os::unix::prelude::OwnedFd;
use std::{io, os::unix::fs::DirEntryExt2}; 
use std::path::Path; 
use std::process; 
use serialport::{SerialPortBuilder, SerialPort}; 

const DEV_PATH: Path = Path::new("/dev"); 

fn _find_arduino_tty() -> io::Result<dyn SerialPort> {
    for device_entry in DEV_PATH.read_dir()? {
        if let Err(_) = device_entry { continue; }
        
        // Convert to UNIX fd, then call into glibc for `istty` check
        let device_entry = device_entry.unwrap(); 
        
        // If fd is tty, fork proc `udevadm` to check if belongs to Arduino

        // If tty belongs to Arduino, handshake (maybe try diff. baud rates?)

        // If handshake successful, return `dyn SerialPort`
        todo!()
    }
}