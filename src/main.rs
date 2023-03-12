#![allow(dead_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::io;
use std::io::Write;
use std::time::Duration;
use std::thread::sleep;

use serialport::{SerialPortType, SerialPort};
use serial_communicator::{read_string_until_byte, write_str_ends_with};
use log::{error, info};

mod util;

const DEFAULT_BAUD_RATE: u32 = 115_200;
const HANDSHAKE_TXM: &str = "HELLO ARDUINO";
const HANDSHAKE_RXM: &str = "HELLO RASPI";

const NULL_TERM: u8 = 0x00;
const LF_TERM:   u8 = b'\n';
const EOF_TERM:  u8 = 0x04;

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
    const _FN_NAME: &str = "[serial-communicator::_find_arduino_serialport]";
    stderrlog::new().module(module_path!()).init().unwrap();

    let available_ports = serialport::available_ports()?;
    for info in &available_ports {
        if let SerialPortType::UsbPort(_) = &info.port_type {
            // Do not check for metadata, which enables 3rd party boards to be used
            let port = serialport::new(&info.port_name, baud_rate)
                .timeout(Duration::from_millis(100))
                .open();
            if port.is_err() { continue; } // Cannot open port
            let mut port = port.unwrap();
           
            /* Handshake */
            let res = serial_communicator::write_str_ends_with(
                port.as_mut(),
                HANDSHAKE_TXM,
                LF_TERM
            );
            if res.is_ok() { // => Can send
                match serial_communicator::read_string_until_byte(
                    port.as_mut(),
                    LF_TERM
                ) {
                    Ok(msg) => {
                        // => Can receive -- Let the Arduino check for correctness too [TODO]
                        let msg = &msg.as_bytes()[..msg.as_bytes().len() - 1];
                        if msg == HANDSHAKE_RXM.as_bytes() {
                           // => Received correctly
                           return Ok(port);
                        }
                        // => Received incorrectly (maybe change baud rate?)
                        continue;
                    },
                    _ => // continue, // => Cannot receive
                    return Ok(port), // Currently no handshake impl, return as well
                }
            }
        }
    }

    return Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("{_FN_NAME} Cannot find Arduino ttyusb device at baud rate `{baud_rate}`")
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

    /* 1. Find Arduino device */
    let mut arduino_port = match _find_arduino_serialport(DEFAULT_BAUD_RATE) {
        Ok(p) => p,
        Err(e) => {
            // => Cannot find arduino ttyusb @ given baud rate, return
            error!("{}", e);
            return;
        }
    };
    let mut buffer: String = String::with_capacity(4096);
    let mut eof_flag = false;

    sleep(Duration::from_secs(3));

    loop {
        /* 2. Read from `stdin` and re-send to Arduino */
        buffer.clear();
        match io::stdin().read_line(&mut buffer) {
            Ok(0) => {
                // => EOF reached, send once and close
                info!("{} EOF reached at stdin", _FN_NAME);
                eof_flag = true;
            },
            Ok(_) => (),
            Err(e) => {
                error!("{} Unexpected error when reading from stdin: \n{:#?}", _FN_NAME, e);
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
            Ok(s) =>
            if let Err(e) = io::stdout().write_all(&s.as_bytes()[0..s.len() - 1]) {
                error!("{} Unexpected error when writing to stdout: \n{:#?}", _FN_NAME, e);
                return;
            },
            Err(e) => {
                error!("{} Unexpected error when reading from arduino tty: \n{:#?}", _FN_NAME, e);
                return;
            }
        }
       
        if eof_flag { return; }
    }
}