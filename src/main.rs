extern crate rosc;
extern crate docopt;
extern crate rustc_serialize;
extern crate portmidi as midi;
extern crate rsoundio;

use std::net::{Ipv4Addr, UdpSocket, AddrParseError};
use std::io;
use std::str::FromStr;
use std::thread;
use std::sync::mpsc; // multiple producer/single consumer

/// `r#"..."` are so called *raw* strings (don't need to be escaped)
const USAGE: &'static str = r#"
Ytterbium OSC controllable synthesizer

Usage:
    ytterbium [--in-port=<port> --out-port=<port> --addr=<addr> --sample-rate=<sr>]
    ytterbium (-h | --help | --version)

Options:
    --in-port=<port>    OSC listening port. [default: 9090]
    --out-port=<port>   OSC listening port. [default: 9091]
    --addr=<addr>       Network interface to listen on. [default: 0.0.0.0]
    --sample-rate=<sr>  Playback sample rate. [default: 48000]
    -h --help           Show this help page.
    --version           Print the version number and exit.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_in_port: usize,
    flag_out_port: usize,
    flag_addr: String,
    flag_sample_rate: usize,
    flag_help: bool,
    flag_version: bool,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn check(args: Args) -> Result<Args, docopt::Error> {
    if args.flag_in_port > 65536 || args.flag_out_port > 65536 {
        Err(docopt::Error::Decode("Port out of range, must be in [0, 65535]".to_owned()))
    } else {
        Ok(args)
    }
}

fn main() {
    let argv: Vec<String> = ::std::env::args().collect();
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|docopt| {
                             let version = Some(format!("ytterbium {}", VERSION));
                             docopt.version(version)
                                   .argv(argv.into_iter())
                                   .decode::<Args>()
                         })
                         .and_then(|args| check(args))
                         .unwrap_or_else(|err| err.exit());
    run(args).map_err(|err| println!("{:?}", err));
    println!("Exiting ...");
}

#[derive(Debug)]
enum RunError {
    Unknown,
    Unimplemented,
    AddrError(AddrParseError),
    SocketError(io::Error),
    OscError(rosc::OscError),
    MidiError(midi::PortMidiError),
    ThreadError(String),
    AudioError(rsoundio::SioError),
}

pub enum ControlEvent {
    Osc(rosc::OscPacket),
    Midi(midi::MidiEvent),
}

fn osc_receiver(socket: UdpSocket, tx: mpsc::Sender<ControlEvent>) -> Result<(), RunError> {
    let mut buf = [0u8; rosc::decoder::MTU];
    loop {
        let (size, addr) = try!(socket.recv_from(&mut buf)
                                      .map_err(|err| RunError::SocketError(err)));
        match rosc::decoder::decode(&buf).map_err(|err| RunError::OscError(err)) {
            Ok(packet) => tx.send(ControlEvent::Osc(packet)).unwrap(),
            Err(e) => println!("Osc packet decoding error: {:?}", e),
        }
    }
}

fn midi_receiver(tx: mpsc::Sender<ControlEvent>) -> Result<(), RunError> {
    try!(midi::initialize().map_err(|err| RunError::MidiError(err)));
    match midi::count_devices() as usize {
        0 => {
            println!("No Midi device found");
            Ok(())
        }
        device_cnt @ _ => {
            let mut devices: Vec<midi::DeviceInfo> = Vec::with_capacity(device_cnt);
            for i in 0..device_cnt {
                midi::get_device_info(i as i32).map(|info| devices.push(info));
            }
            println!("Found the following midi-devices:");
            for device in devices {
                println!("\tid: {}, name: {}", device.device_id, device.name);
            }
            midi::terminate().map_err(|err| RunError::MidiError(err))
        }
    }
}

fn udp_socket(args: &Args) -> Result<UdpSocket, RunError> {
    let ipv4_addr = try!(Ipv4Addr::from_str(&args.flag_addr)
                             .map_err(|err| RunError::AddrError(err)));
    UdpSocket::bind((ipv4_addr, args.flag_in_port as u16)).map_err(|err| RunError::SocketError(err))
}

fn event_router(rx: mpsc::Receiver<ControlEvent>) {
    loop {
        match rx.recv().unwrap() {
            ControlEvent::Osc(packet) => println!("osc: {:?}", packet),
            ControlEvent::Midi(msg) => println!("midi: {:?}", msg),
        }
    }
}

fn run(args: Args) -> Result<(), RunError> {
    let socket = try!(udp_socket(&args));

    let (tx, rx) = mpsc::channel();
    let osc_tx = tx.clone();
    let osc = thread::Builder::new()
                  .name("osc".to_owned())
                  .spawn(move || -> Result<(), RunError> { osc_receiver(socket, osc_tx) })
                  .unwrap();

    let midi_tx = tx.clone();
    let midi = thread::Builder::new()
                   .name("midi".to_owned())
                   .spawn(move || -> Result<(), RunError> { midi_receiver(midi_tx) })
                   .unwrap();

    let router = thread::Builder::new()
                     .name("router".to_owned())
                     .spawn(move || event_router(rx))
                     .unwrap();

    let sio = rsoundio::SoundIo::new();
    // connect to default backend
    sio.connect();
    sio.flush_events();
    let dev = sio.default_output_device().unwrap();
    let mut out = dev.create_outstream().unwrap();
    // TODO: implement audio output in main thread

    let res = osc.join();
    res.unwrap()
}
