//! Parse input from stdin and log actions on stdout
extern crate televerknet;

use std::io::{self, Read};

/// A type implementing Perform that just logs actions
struct Log;

impl televerknet::Perform for Log {
    fn data(&mut self, intermediates: &[u8], ignoring: bool) {
        println!(
            "[data] intermediate={:?}, ignoring={:?}",
            intermediates, ignoring
        );
    }

    fn execute(&mut self, byte: u8) {
        println!("[execute] {:02x}", byte);
    }

    fn iac_dispatch(&mut self, byte: u8) {
        println!("[iac_dispatch] {:02x}", byte);
    }

    fn sub_dispatch(&mut self, subs: &[u8]) {
        println!("[sub_dispatch] {:?}", subs);
    }

    fn negotiate_dispatch(&mut self, cmd: u8, opt: u8) {
        println!("[negotiate_dispatch] cmd={:02x}, opt={:02x}", cmd, opt);
    }

    fn subnegotiate_dispatch(&mut self, params: &[u8], opt: u8) {
        println!(
            "[subnegotiate_dispatch] params={:?}, opt={:02x}",
            params, opt
        );
    }

    fn zmp_dispatch(&mut self, params: &[&[u8]]) {
        println!("[zmp_dispatch] {:?}", params);
    }

    fn ttypes_dispatch(&mut self, cmd: u8, terminal_type: &[u8]) {
        println!(
            "[ttypes_dispatch] cmd={:02x}, terminal_type={:?}",
            cmd, terminal_type
        );
    }

    fn compress_dispatch(&mut self, state: u8) {
        println!("[compress_dispatch] {:02x}", state);
    }
}

fn main() {
    let input = io::stdin();
    let mut handle = input.lock();

    let mut statemachine = televerknet::Parser::new();
    let mut parser = Log;

    let mut buf: [u8; 2048] = unsafe { std::mem::uninitialized() };

    loop {
        match handle.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for byte in &buf[..n] {
                    statemachine.advance(&mut parser, *byte);
                }
            }
            Err(err) => {
                println!("err: {}", err);
                break;
            }
        }
    }
}
