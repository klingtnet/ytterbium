extern crate rosc;
extern crate docopt;
extern crate rustc_serialize;
extern crate rsoundio;
extern crate rb;

use std::f32::consts::PI as PI32;
use std::thread;
use std::sync::mpsc; // multiple producer/single consumer

use rb::{RB, RbProducer, RbConsumer};

mod errors;
use errors::RunError;

mod event;
use event::receiver::{Receiver, OscReceiver, MidiReceiver, RawControlEvent};
use event::router::{EventRouter, ControlEvent};

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
    --debug                 Print debugging information.
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
    flag_debug: bool,
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

fn run(args: Args) -> Result<(), RunError> {
    let buf = rb::SpscRb::new(4096);
    let (producer, consumer) = (buf.producer(), buf.consumer());
    let (tx_receiver, rx_router) = mpsc::channel();
    let (tx_router, rx_dsp) = mpsc::channel();
    let mut osc_receiver = try!(OscReceiver::new(args.flag_addr,
                                                 args.flag_in_port as u16,
                                                 tx_receiver.clone()));
    // let mut midi_receiver = try!(MidiReceiver::new(tx_receiver.clone()));
    let event_router = EventRouter::<RawControlEvent, ControlEvent>::new(rx_router, tx_router);

    let osc = thread::Builder::new()
                  .name("osc".to_owned())
                  .spawn(move || osc_receiver.receive_and_send())
                  .unwrap();

    // let _ = thread::Builder::new().name("midi".to_owned()).spawn(move || {}).unwrap();

    let _ = thread::Builder::new()
                .name("router".to_owned())
                .spawn(move || event_router.route())
                .unwrap();

    let _ = thread::Builder::new()
                .name("dsp".to_owned())
                .spawn(move || {
                    let w = (2.0 * PI32 * 440.0) / 48_000.0;
                    let mut n = 0;

                    loop {
                        // TODO: busy wait loop, should be not so bad when the actual dsp
                        // calculations take place

                        // TODO: dsp and audio output are going to need shared audio buffer
                        if let Ok(msg) = rx_dsp.try_recv() {

                        }
                        let data = (0..512)
                                       .map(|x| {
                                           n = (n + 1) % 110;
                                           (w * n as f32).sin() * 0.5
                                       })
                                       .collect::<Vec<f32>>();
                        let cnt = producer.write_blocking(&data).unwrap();
                    }
                })
                .unwrap();

    let mut sio = rsoundio::SoundIo::new();
    sio.set_name("ytterbium").unwrap();
    // connect to default backend
    sio.connect().unwrap();
    println!("Connected to: {}", sio.current_backend().unwrap());
    sio.flush_events();
    let dev = sio.default_output_device().unwrap();
    let mut out = dev.create_outstream().unwrap();
    out.set_name("debug").ok();
    out.set_format(rsoundio::SioFormat::Float32LE).unwrap();
    println!("Format: {}", out.format().unwrap());
    out.register_write_callback(|out: rsoundio::OutStream,
                                 min_frame_count: u32,
                                 max_frame_count: u32| {
        // TODO: pulseaudio has problems with buffer sizes smaller than 2048
        let mut data = vec![0.0f32; 2048];
        let cnt = consumer.read_blocking(&mut data).unwrap();
        let frames = vec![data.clone(), data.clone()];
        out.write_stream_f32(min_frame_count, &frames).unwrap();
    });
    out.register_underflow_callback(|out: rsoundio::OutStream| {
         println!("Underflow in {} occured!", out.name().unwrap())
    });
    out.open().unwrap();
    out.start().unwrap();
    osc.join();
    Ok(())
}
