#![allow(dead_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::io;
use std::io::Write; 
use std::time::Duration;
use std::thread::sleep;

use serialport::{SerialPortType, SerialPort};
use serial_communicator::Request; 
use log::{error, info};

mod util;
mod bindings;

use util::serial_helper::*; 

const BAUD_RATE_OPTIONS: [u32; 2] = [115_200, 9_600]; 

/// Tries to connect to relevant Arduino tty devices (i.e., all Arduinos connected to host). 
///
/// ### Returns
/// - `Ok(ports)` which encapsulates `Vec` of `dyn SerialPort` trait objects.
/// - `Err(io::Error)` which is of kind `io::ErrorKind::NotFound`, indicating that no suitable `tty`
///    devices could be found.
fn _find_arduino_serialports() -> io::Result<Vec<Box<dyn SerialPort>>> {
    const _FN_NAME: &str = "[serial-communicator::find_arduino_serialport]";

    let mut port_buf: Vec<Box<dyn SerialPort>> = Vec::with_capacity(2); 
    let available_ports = serialport::available_ports()?;
    for info in &available_ports {
        if let SerialPortType::UsbPort(_) = &info.port_type {
            // Do not check for metadata, which enables 3rd party boards to be used
            for baud_rate in BAUD_RATE_OPTIONS {
                let port = serialport::new(&info.port_name, baud_rate)
                    .timeout(Duration::from_secs(1))
                    .flow_control(serialport::FlowControl::None)
                    .open();
                if port.is_err() { continue; } // Cannot open port
                let port = port.unwrap();

                // Give time for Arduino to reset connection
                sleep(Duration::from_secs(3)); 

                port_buf.push(port); 
            }
        }
    }

    if port_buf.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{_FN_NAME} No Arduino `tty` device connected to host!")
        ));
    } else {
        return Ok(port_buf); 
    }
}

/// Communicator which works in a WRITE-READ loop. 
/// Assumming Cosmos' ctrl loop it should be sufficient? 
fn main() {
    const _FN_NAME: &str = "[serial-communicator::main]";
    simple_logger::init_with_env().unwrap(); 

    /* 1. Find Arduino devices */
    let mut arduino_ports = match _find_arduino_serialports() {
        Ok(p) => p,
        Err(e) => {
            // => Cannot find arduino ttyusb @ given baud rate, return
            error!("{}", e);
            return;
        }
    };
    // [TODO] Currently this would be the sole Arduino connected. No idea how many is actually used! 
    let arduino_port: &mut dyn SerialPort = arduino_ports[0].as_mut(); 

    info!("{_FN_NAME} Connected to Arduino"); 
    let mut action_buffer: String  = String::with_capacity(512);
    let mut read_buffer:   Vec<u8> = vec![0; 512]; 
    
    loop {
        /* 2. Read from `stdin` and re-send to Arduino */
        action_buffer.clear();
        let action; 
        match io::stdin().read_line(&mut action_buffer) {
            Ok(0) => {
                // => EOF reached, close pipe
                info!("{_FN_NAME} EOF reached at stdin");
                return; 
            },
            Ok(_) => {
                // => Try convert to `Action` instance
                action = Request::try_from(action_buffer.as_ref())
            },
            Err(e) => {
                error!("{_FN_NAME} Unexpected error when reading from stdin: \n{:#?}", e);
                return;
            }
        };

        match action {
            Ok(Request::Read) => {
                // => Wait read on Arduino, send to `stdout`
                while let Err(e) = read_all_bytes_into(
                    arduino_port, 
                    &mut read_buffer
                ) { 
                    if e.kind() == std::io::ErrorKind::TimedOut { continue; }
                    error!(
                        "{_FN_NAME} Unexpected error when reading from Arduino: \n{:#?}", 
                        e
                    ); 
                    break; 
                }
                let mut stdout = io::stdout(); 
                if let Err(e) = stdout.write_all(&read_buffer) {
                    error!(
                        "{_FN_NAME} Unexpected error when writing to stdout: \n{:#?}", 
                        e
                    ); 
                    return; 
                }
                if let Err(e) = stdout.flush() {
                    error!(
                        "{_FN_NAME} Unexpected error when flushing stdout: \n{:#?}", 
                        e
                    ); 
                    return; 
                } 
                info!(
                    "{_FN_NAME} Received \"{:x?}\"", 
                    read_buffer
                ); 
                read_buffer.clear(); 
            }, 
            Ok(Request::Write(v)) => {
                // => Write to Arduino
                if let Err(e) = write_all_bytes(
                    arduino_port, 
                    &v, 
                ) {
                    error!(
                        "{} Unexpected error when sending to arduino tty: \n{:#?}", 
                        _FN_NAME, 
                        e
                    );
                    return;
                }
                if let Err(e) = arduino_port.flush() {
                    error!(
                        "{_FN_NAME} Unexpected error when flushing arduino tty: \n{:#?}", 
                        e
                    ); 
                    return; 
                }
                info!(
                    "{_FN_NAME} Written {:x?}", 
                    v
                ); 
            }, 
            Err(e) => 
                error!("{_FN_NAME} Invalid input from stdin: \n{:#?}", e), 
        }
    }

    // arduino_port.clear(serialport::ClearBuffer::All); 
}
