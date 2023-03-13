#![allow(dead_code)]
#![cfg(unix)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::io;
use std::io::Write;
use std::time::Duration;
use std::thread::sleep;
// use std::fs::File; 

use serialport::{SerialPortType, SerialPort};
use serial_communicator::{read_string_until_byte, write_str_ends_with};
use log::{error, info};

mod util;

const DEFAULT_BAUD_RATE: u32 = 115_200;
const LF_TERM: u8 = b'\n'; 

/// Tries to connect to the first *correctly set* Arduino connected via USB as `tty` device.
///
/// ### Arguments
/// - `baud_rate` for Arduino
///
/// ### Returns
/// - `Ok(port)` which encapsulates a dynamically dispatched `SerialPort` trait object.
/// - `Err(io::Error)` which is of kind `io::ErrorKind::NotFound`, indicating that a suitable `tty`
///    device cannot be found.
fn find_arduino_serialport(baud_rate: u32) -> io::Result<Box<dyn SerialPort>> {
    const _FN_NAME: &str = "[serial-communicator::find_arduino_serialport]";

    let available_ports = serialport::available_ports()?;
    for info in &available_ports {
        if let SerialPortType::UsbPort(_) = &info.port_type {
            // Do not check for metadata, which enables 3rd party boards to be used
            let port = serialport::new(&info.port_name, baud_rate)
                .timeout(Duration::from_secs(10))
                .flow_control(serialport::FlowControl::None)
                .open();
            if port.is_err() { continue; } // Cannot open port
            let mut port = port.unwrap();

            // Give time for Arduino to reset connection
            sleep(Duration::from_secs(3)); 
            
            if let Ok(_) = handshake(port.as_mut()) {
                return Ok(port); 
            }
        }
    }

    return Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("{_FN_NAME} Cannot find Arduino ttyusb device at baud rate `{baud_rate}`")
    ));
}

fn handshake(port: &mut dyn SerialPort) -> io::Result<()> {
    const _FN_NAME: &str = "[serial-communicator::handshake]"; 
    const HANDSHAKE_TX_MSG: &str = "PC TO ARDUINO_1";
    const HANDSHAKE_RX_MSG: &str = "ARDUINO_1 TO PC";

    for i in 1..=10 {
        info!("{_FN_NAME} Waiting for Arduino: {i}/10..."); 

        /* 1. Send TX to Arduino */
        let write_tx_res = serial_communicator::write_str_ends_with(
            port, 
            HANDSHAKE_TX_MSG, 
            LF_TERM
        ); 
        if let Err(e) = write_tx_res {
            error!("{_FN_NAME} Cannot write tx msg to Arduino: {:#?}; {i}/10", e); 
            continue; 
        }

        /* 2. Receive RX from Arduino */
        let read_rx_res = serial_communicator::read_string_until_byte(
            port, 
            LF_TERM
        ); 
        match read_rx_res {
            Err(e) => {
                error!("{_FN_NAME} Cannot read rx msg from Arduino: {:#?}; {i}/10", e); 
                continue; 
            }, 
            Ok(s) if &s[..s.len() - 1] == HANDSHAKE_RX_MSG => (), 
            Ok(_) => {
                error!("{_FN_NAME} Mismatched rx msg from Arduino; {i}/10"); 
                continue; 
            }
        }

        /* 3. Send back RX to Arduino */
        let write_rx_res = serial_communicator::write_str_ends_with(
            port, 
            HANDSHAKE_RX_MSG, 
            LF_TERM
        ); 
        if let Err(e) = write_rx_res {
            error!("{_FN_NAME} Cannot write back rx msg to Arduino: {:#?}; {i}/10", e); 
            continue; 
        }

        return Ok(()); 
    }
    return Err(io::Error::new(
        io::ErrorKind::ConnectionRefused, 
        format!("{_FN_NAME} Cannot handshake with port device.")
    )); 
}

/// Loops over four things:
/// - Wait read from stdin (e.g., move)
/// - send to arduino
/// - wait read from arduino (e.g., reed switches)
/// - send to stdout
///
/// That's it. Returns whenever stdin reaches EOF e.g., after parent proc closes pipe.
///
/// ### TODO
/// - Arduino-side software needs to implement handshake abilities.
/// - For now `main` loops over four things. Ideally (to better fit `master-program::main` control
///   loop) `main` could instead cache over reads and writes and return/send values on-demand, as
///   sent alongside stdin. The current impl is much less messy though.
/// - `_find_arduino_serialport` can maybe be expanded to accept multiple baud rates for handshaking
///   purposes. This is not too high of a concern, though.
/// - It works on real Arduino (ofc.) but buffering needs work maybe -- this can also affect
///   handshaking impl.
fn main() {
    const _FN_NAME: &str = "[serial-communicator::main]";
    stderrlog::new().module(module_path!()).init().unwrap();
    // let mut file = File::create("out.txt").unwrap(); 

    /* 1. Find Arduino device */
    let mut arduino_port = match find_arduino_serialport(DEFAULT_BAUD_RATE) {
        Ok(p) => p,
        Err(e) => {
            // => Cannot find arduino ttyusb @ given baud rate, return
            error!("{}", e);
            return;
        }
    };
    info!("{_FN_NAME} Connected to Arduino"); 
    let mut buffer: String = String::with_capacity(4096);
    let mut eof_flag = false;
    
    loop {
        /* 2. Read from `stdin` and re-send to Arduino */
        buffer.clear();
        match io::stdin().read_line(&mut buffer) {
            Ok(0) => {
                // => EOF reached, send once and close
                info!("{_FN_NAME} EOF reached at stdin");
                eof_flag = true;
            },
            Ok(_) => (),
            Err(e) => {
                error!("{_FN_NAME} Unexpected error when reading from stdin: \n{:#?}", e);
                return;
            }
        };
        match write_str_ends_with(
            arduino_port.as_mut(),
            &buffer,
            LF_TERM
        ) {
            Ok(_) => (),
            Err(e) => {
                error!("{} Unexpected error when sending to arduino tty: \n{:#?}", _FN_NAME, e);
                return;
            }
        }

        /* 3. Wait read on Arduino, send to `stdout` */
        match read_string_until_byte(arduino_port.as_mut(), LF_TERM) {
            Ok(s) => {
                // file.write_all(&s.as_bytes()).unwrap(); 
                if let Err(e) = io::stdout().write_all(&s.as_bytes()[..s.len() - 1]) {
                    error!("{} Unexpected error when writing to stdout: \n{:#?}", _FN_NAME, e);
                    return;
                }
            },
            Err(e) => {
                error!("{} Unexpected error when reading from arduino tty: \n{:#?}", _FN_NAME, e);
                return;
            }
        }
       
        if eof_flag { return; }
    }
}