#![allow(dead_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::io;
use std::io::Write; 
use std::time::Duration;
use std::thread::sleep;

use serialport::{SerialPortType, SerialPort};
use serial_communicator::Action; 
use serial_communicator::{read_into_string_buffer, write_str_ends_with};
use log::{error, info};

mod util;

const BAUD_RATE_OPTIONS: [u32; 2] = [115_200, 9_600]; 
const LF_TERM: u8 = b'\n'; 

/// Tries to connect to relevant Arduino tty devices (i.e., all Arduinos connected to host). 
///
/// ### Returns
/// - `Ok(ports)` which encapsulates `Vec` of `dyn SerialPort` trait objects.
/// - `Err(io::Error)` which is of kind `io::ErrorKind::NotFound`, indicating that no suitable `tty`
///    devices could be found.
fn find_arduino_serialports() -> io::Result<Vec<Box<dyn SerialPort>>> {
    const _FN_NAME: &str = "[serial-communicator::find_arduino_serialport]";

    let mut port_buf: Vec<Box<dyn SerialPort>> = Vec::with_capacity(2); 
    let available_ports = serialport::available_ports()?;
    for info in &available_ports {
        if let SerialPortType::UsbPort(_) = &info.port_type {
            // Do not check for metadata, which enables 3rd party boards to be used
            for baud_rate in BAUD_RATE_OPTIONS {
                let port = serialport::new(&info.port_name, baud_rate)
                    .timeout(Duration::from_secs(10))
                    .flow_control(serialport::FlowControl::None)
                    .open();
                if port.is_err() { continue; } // Cannot open port
                let mut port = port.unwrap();

                // Give time for Arduino to reset connection
                sleep(Duration::from_secs(3)); 
                
                if let Ok(_) = handshake(port.as_mut()) {
                    port.set_timeout(Duration::from_secs(1))?; 
                    port_buf.push(port); 
                } 
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

/// Raspi-side handshake implementation. 
/// 
/// For an Arduino-side sample impl, refer to `example_handshake_impl` which might expand to a 
/// whole header in the future. 
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

        eprintln!("{}", port.bytes_to_read().unwrap()); 
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
        
        let read_successful_msg = serial_communicator::read_string_until_byte(
            port, 
            LF_TERM
        ); 
        if let Err(e) = read_successful_msg {
            error!("{_FN_NAME} Unexpected Arduino tty connection drop: {:#?}; {i}/10", e); 
            continue; 
        }
        info!("{_FN_NAME} {}", read_successful_msg.unwrap().trim_end()); 
        return Ok(()); 
    }
    return Err(io::Error::new(
        io::ErrorKind::ConnectionRefused, 
        format!("{_FN_NAME} Cannot handshake with port device.")
    )); 
}

/// Communicator which works in a WRITE-READ loop. 
/// Assumming Cosmos' ctrl loop it should be sufficient? 
/// 
/// ### Upcoming FIXME...
/// - Timeout handling
fn main() {
    const _FN_NAME: &str = "[serial-communicator::main]";
    simple_logger::init_with_env().unwrap(); 

    /* 1. Find Arduino devices */
    let mut arduino_ports = match find_arduino_serialports() {
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
    let mut action_buffer: String = String::with_capacity(256);
    let mut read_buffer:   String = String::with_capacity(256); 
    
    /* Obtain stdout stream lock */
    let mut stdout = io::stdout().lock(); 
    
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
                action = Action::try_from(action_buffer.as_ref())
            },
            Err(e) => {
                error!("{_FN_NAME} Unexpected error when reading from stdin: \n{:#?}", e);
                return;
            }
        };

        match action {
            Ok(Action::Read) => {
                // => Wait read on Arduino, send to `stdout`
                if let Err(_) = read_into_string_buffer(
                    arduino_port, 
                    LF_TERM, 
                    &mut read_buffer
                ) { 
                    return; 
                }
                if let Err(e) = stdout.write_all(read_buffer.as_bytes()) {
                    error!(
                        "{_FN_NAME} Unexpected error when writing to stdout: \n{:#?}", 
                        e
                    ); 
                    return; 
                }
                read_buffer.clear(); 
            }, 
            Ok(Action::Write(s)) => {
                // => Write to Arduino
                if let Err(e) = write_str_ends_with(
                    arduino_port, 
                    &s, 
                    LF_TERM
                ) {
                    error!(
                        "{} Unexpected error when sending to arduino tty: \n{:#?}", 
                        _FN_NAME, 
                        e
                    );
                    return;
                }
            }, 
            Err(e) => 
                error!("{_FN_NAME} Invalid input from stdin: \n{:#?}", e), 
        }
    }
}
