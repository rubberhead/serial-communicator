#![allow(dead_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::io::{self, ErrorKind};
use std::io::Write; 
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio_serial::SerialStream;
use log::{error, info};

use serial_communicator::{Request, Instruction}; 

mod util;
mod bindings;

const BAUD_RATE: u32 = 115_200; 

fn _find_devices() -> Vec<SerialStream> {
    const _FN_NAME: &str = "[serial-communicator::_find_devices]";

    let mut port_buf: Vec<SerialStream> = Vec::new(); 
    if let Ok(ports) = tokio_serial::available_ports() {
        for port_info in ports {
            let port = tokio_serial::new(port_info.port_name, BAUD_RATE)
                .timeout(Duration::from_secs(1)); 
            if let Ok(port) = SerialStream::open(&port) {
                port_buf.push(port); 
            }
        }
    }
    return port_buf; 
}

async fn write_and_wait_response(
    port_stream: &mut SerialStream, 
    instruction: Instruction
) -> io::Result<(Instruction, usize)> {
    const _FN_NAME: &str = "[serial-communicator::write_and_wait_response]"; 

    port_stream.writable().await?; 
    match port_stream.try_write(&instruction) {
        Ok(_) => {
            AsyncWriteExt::flush(port_stream).await?; 
        }, 
        Err(e) => {
            error!(
                "{_FN_NAME} Unexpected error when writing to port_stream: \n{:#?}", 
                e
            ); 
            return Err(e); 
        }
    }

    port_stream.readable().await?; 
    let mut response_buf: Vec<u8> = vec![0; 8]; 
    let mut res = port_stream.try_read(&mut response_buf); 
    while let Err(e) = res {
        if e.kind() == ErrorKind::WouldBlock {
            // => Continue at block
            // [TODO] Timeout?
            res = port_stream.try_read(&mut response_buf); 
            continue; 
        }
        return Err(e); 
    }
    let read_amnt = res.unwrap(); 
    info!("{_FN_NAME} Received {:x?}", &response_buf[..read_amnt]); 
    return Ok((response_buf, read_amnt)); 
}

#[tokio::main]
async fn main() {
    const _FN_NAME: &str = "[serial-communicator::main]";
    simple_logger::init_with_env().unwrap(); 

    /* 1. Find Arduino device -- ONE device */
    let mut port_streams = _find_devices(); 
    if port_streams.is_empty() {
        error!("{_FN_NAME} Cannot find serial devices. Quitting..."); 
        return; 
    }
    let mut port_stream = port_streams.pop().unwrap(); 
    info!("{_FN_NAME} Connected to Arduino"); 

    let mut action_buffer: String  = String::with_capacity(1024);
    // let mut read_buffer:   Vec<u8> = vec![0; 1024]; 
    
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
            Ok(Request::Write(v)) => {
                // => Write to Arduino, then wait on response and send to stdout
                match write_and_wait_response(&mut port_stream, v).await {
                    Ok((response, response_len)) => {
                        if let Err(e) = io::stdout().write_all(&response[..response_len]) {
                            error!("{_FN_NAME} WRITE: Unexpected error when writing to stdout: \n{:#?}", e); 
                            return; 
                        }
                        if let Err(e) = io::stdout().flush() {
                            error!("{_FN_NAME} WRITE: Unexpected error when flushing stdout: \n{:#?}", e); 
                            return; 
                        }
                    }, 
                    Err(e) => {
                        error!("{_FN_NAME} WRITE: Unexpected error when requesting Arduino: \n{:#?}", e); 
                        return; 
                    }
                }  
            }, 
            Err(e) => 
                error!("{_FN_NAME} Invalid input from stdin: \n{:#?}", e), 
        }
    }
}
