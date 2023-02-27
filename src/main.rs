#![cfg(unix)]
#![allow(dead_code)]

use std::io; 
use std::time::Duration;

use serialport::{SerialPortType, SerialPort}; 

mod util; 

const DEFAULT_BAUD_RATE: u32 = 9600; 
const HANDSHAKE_TXM: &str = "HELLO ARDUINO"; 
const HANDSHAKE_RXM: &str = "HELLO RASPI"; 

const NULL_TERM: u8 = unsafe { *(&'\0' as *const char).cast() }; 
const LF_TERM:   u8 = unsafe { *(&'\n' as *const char).cast() }; 

/// Tries to connect to the first *correctly set* Arduino connected via USB as `tty` device. 
/// 
/// ### Arguments
/// - `baud_rate` for Arduino
/// 
/// ### Returns
/// - `Ok(port)` which encapsulates a dynamically dispatched `SerialPort` trait object.
/// - `Err(io::Error)` which is of kind `io::ErrorKind::NotFound`, indicating that a suitable `tty` 
///    device cannot be found. 
fn _find_arduino_serialport(baud_rate: u32) -> io::Result<Box<dyn SerialPort>> {
    let available_ports = serialport::available_ports()?; 
    for info in &available_ports {
        if let SerialPortType::UsbPort(_) = &info.port_type {
            // Do not check for metadata, which enables 3rd party boards to be used
            let port = serialport::new(&info.port_name, baud_rate)
                .timeout(Duration::from_millis(100))
                .open();
            if let Err(_) = port { continue; } // Cannot open port 
            let mut port = port.unwrap(); 
            
            /* Handshake */
            if let Ok(_) = serial_communicator::write_str_ends_with(
                port.as_mut(), 
                HANDSHAKE_TXM, 
                LF_TERM
            ) { // Can send
                match serial_communicator::read_string_until_byte(
                    port.as_mut(), 
                    LF_TERM
                ) {
                    Ok(msg) => {
                        // Can receive -- Let the Arduino check for correctness too [TODO]
                        let msg = &msg.as_bytes()[..msg.as_bytes().len() - 1]; 
                        if msg == HANDSHAKE_RXM.as_bytes() {
                           // Received correctly
                           return Ok(port); 
                        }
                        continue; // Received incorrectly (maybe change baud rate?)
                    }, 
                    _ => continue, // Cannot receive
                }
            }
        }
    }
    return Err(io::Error::new(
        io::ErrorKind::NotFound, 
        "[main::_find_arduino_serialport] Cannot find Arduino ttyusb device with given configuration"
    )); 
}

/* <COPE> It will be useful
fn _find_arduino_tty() -> io::Result<Box<dyn SerialPort>> { 
    for device_entry in Path::new("/dev").read_dir()? {
        if let Err(_) = device_entry { continue; }
        
        /* Convert to UNIX fd, then call into glibc for `istty` check */
        let device_path = device_entry.unwrap().path().into_boxed_path(); 
        let device_file: Option<File> = File::open(&device_path).ok(); 
        // `Err` most likely because it's dir or occupied, which is fine. 
        // If that is due to no permission though -- prob. a sign of big problem. 
        if let None = device_file { continue; }
        unsafe {
            let device_fd: c_int = device_file.unwrap().as_raw_fd(); 
            let device_fd_is_tty: i32 = libc::isatty(device_fd); 
            if device_fd_is_tty != 1 { continue; } // See `man isatty`.
        }
        
        /* If fd is tty, fork proc `udevadm` to check if belongs to Arduino */
        let udevadm_out = process::Command::new("udevadm")
            .args(["info", "-q", "symlink"]).arg(device_path.as_os_str())
            .spawn().expect("[main::_find_arduino_tty] Failed to execute `udevadm`")
            .wait_with_output()?; 
        if !util::subslice_of("arduino".as_bytes(), &udevadm_out.stdout) { continue; }

        // If tty belongs to Arduino, handshake (maybe try diff. baud rates?)
        

        // If handshake successful, return `dyn SerialPort`
        
    }
    todo!()
} */

fn main() -> ! {
    todo!()
}