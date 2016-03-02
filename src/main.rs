extern crate rosc;
extern crate docopt;
extern crate rustc_serialize;
extern crate rsoundio;

use std::net::{Ipv4Addr, UdpSocket};
use std::str::FromStr;
use std::thread;
use std::sync::mpsc; // multiple producer/single consumer

mod errors;
use errors::RunError;

mod receiver;
mod router;
//use receiver::{OscReceiver, MidiReceiver};

/// `r#"..."` are so called *raw* strings (don't need to be escaped)
const USAGE: &'static str = r#"
Ytterbium OSC controllable synthesizer

Usage:
    ytterbium [--in-port=<port> --out-port=<port> --addr=<addr> --sample-rate=<sr>]
    ytterbium (-h | --help | --version)

Options:
    --in-port=<port>        OSC listening port. [default: 9090]
    --out-port=<port>       OSC listening port. [default: 9091]
    --addr=<addr>           Network interface to listen on. [default: 0.0.0.0]
    --sample-rate=<sr>      Playback sample rate. [default: 48000]
    -v --voices=<voices>    Number of voices [default: 1]
    -h --help               Show this help page.
    --version               Print the version number and exit.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_in_port: usize,
    flag_out_port: usize,
    flag_addr: String,
    flag_sample_rate: usize,
    flag_voices: usize,
    flag_help: bool,
    flag_version: bool,
}

const MAX_VOICES: usize = 24;
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let argv: Vec<String> = ::std::env::args().collect();
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|docopt| {
                             let version = Some(format!("ytterbium {}", VERSION));
                             docopt.version(version)
                                   .argv(argv.into_iter())
                                   .decode::<Args>()
                         })
                         .unwrap_or_else(|err| err.exit());
    run(args).map_err(|err| println!("{:?}", err));
    println!("Exiting ...");
}

fn udp_socket(args: &Args) -> Result<UdpSocket, RunError> {
    let ipv4_addr = try!(Ipv4Addr::from_str(&args.flag_addr)
                             .map_err(|err| RunError::AddrError(err)));
    UdpSocket::bind((ipv4_addr, args.flag_in_port as u16)).map_err(|err| RunError::SocketError(err))
}

fn run(args: Args) -> Result<(), RunError> {
    let socket = try!(udp_socket(&args));

    let (tx_router, rx_router) = mpsc::channel();
    let osc_tx = tx_router.clone();
    let osc = thread::Builder::new()
                  .name("osc".to_owned())
                  .spawn(move || -> Result<(), RunError> { receiver::osc_receiver(socket, osc_tx) })
                  .unwrap();

    let midi_tx = tx_router.clone();
    let _ = thread::Builder::new()
                .name("midi".to_owned())
                .spawn(move || -> Result<(), RunError> { receiver::midi_receiver(midi_tx) })
                .unwrap();

    let (tx_dsp, rx_dsp) = mpsc::channel();
    let _ = thread::Builder::new()
                .name("router".to_owned())
                .spawn(move || router::event_router(rx_router, tx_dsp))
                .unwrap();

    let _ = thread::Builder::new()
                .name("dsp".to_owned())
                .spawn(move || {
                    loop {
                        // TODO: busy wait loop, should be not so bad when the actual dsp
                        // calculations take place

                        // TODO: dsp and audio output are going to need shared audio buffer
                        if let Ok(msg) = rx_dsp.try_recv() {
                            // here comes the dsp code
                        }
                    }
                })
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
