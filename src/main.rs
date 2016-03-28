extern crate rosc;
extern crate docopt;
extern crate rustc_serialize;
extern crate rsoundio;
extern crate rb;

use std::collections::HashMap;
use std::f32::consts::PI as PI32;
use std::thread;
use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::sync::atomic::{AtomicBool, Ordering};

use rb::{RB, RbProducer, RbConsumer};

mod errors;
use errors::RunError;

mod event;
use event::receiver::{Receiver, OscReceiver, MidiReceiver, RawControlEvent};
use event::router::{EventRouter, ControlEvent};

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
    let (tx_osc, tx_midi) = (tx_receiver.clone(), tx_receiver.clone());
    let (tx_router, rx_dsp) = mpsc::channel();
    let dsp_init = Arc::new(Barrier::new(1));
    let audio_init = dsp_init.clone();
    let mut osc_receiver = try!(OscReceiver::new(args.flag_addr, args.flag_in_port as u16));
    let mut midi_receiver = try!(MidiReceiver::new());
    let event_router = EventRouter::<RawControlEvent, ControlEvent>::new(rx_router, tx_router);
    let mut handles = HashMap::with_capacity(5);
    let quit = Arc::new(AtomicBool::new(false));
    let quit_dsp = quit.clone();

    handles.insert("osc",
                   thread::Builder::new()
                       .name("osc".to_owned())
                       .spawn(move || osc_receiver.receive_and_send(tx_osc))
                       .unwrap());

    handles.insert("midi",
                   thread::Builder::new()
                       .name("midi".to_owned())
                       .spawn(move || midi_receiver.receive_and_send(tx_midi))
                       .unwrap());

    handles.insert("router",
                   thread::Builder::new()
                       .name("router".to_owned())
                       .spawn(move || event_router.route())
                       .unwrap());

    handles.insert("dsp",
                   thread::Builder::new()
                       .name("dsp".to_owned())
                       .spawn(move || {
                           const TUNE_FREQ: f32 = 440.0;
                           const SR: f32 = 48000.0;
                           let mut f = 440f32;
                           let mut w = (2.0 * PI32 * f) / SR;
                           let mut n = 0;
                           let mut a = 1.0;

                           dsp_init.wait();
                           loop {
                               if quit_dsp.load(Ordering::Relaxed) {
                                   break;
                               }
                               if let Ok(msg) = rx_dsp.try_recv() {
                                   match msg {
                                       ControlEvent::NoteOn{key, velocity} => {
                                           a = velocity;
                                           f = 2.0f32.powf((key as isize - 69) as f32 / 12.0) *
                                               TUNE_FREQ;
                                           w = (2.0 * PI32 * f) / SR;
                                       }
                                       _ => (),
                                   }
                               }
                               let data = (0..128)
                                              .map(|x| (w * (x + n) as f32).sin() * a)
                                              .collect::<Vec<f32>>();
                               n += 128;
                               let cnt = producer.write_blocking(&data).unwrap();
                           }
                       })
                       .unwrap());

    handles.insert("output",
                   thread::Builder::new()
                       .name("output".to_owned())
                       .spawn(move || {
                           let mut sio = rsoundio::SoundIo::new();
                           sio.set_name("ytterbium").unwrap();
                           // connect to default backend
                           sio.connect().unwrap();
                           println!("Connected to: {}", sio.current_backend().unwrap());
                           sio.flush_events();
                           let dev = sio.default_output_device().unwrap();
                           let mut out = dev.create_outstream().unwrap();
                           out.set_name("debug").ok();
                           // panics when using jack backend
                           out.set_format(rsoundio::SioFormat::Float32LE).unwrap();
                           println!("Format: {}", out.format().unwrap());
                           out.register_write_callback(|out: rsoundio::OutStream,
                                                        min_frame_count: u32,
                                                        max_frame_count: u32| {
                               const LEN: usize = 2048;
                               // TODO: use a length that is not smaller than 2048 for pulseaudio
                               let len = ::std::cmp::min(LEN, max_frame_count as usize);
                               let mut data = vec![0.0f32; LEN];
                               let cnt = consumer.read_blocking(&mut data[..len]).unwrap();
                               let frames = vec![data[..len].iter().cloned().collect(),
                                                 data[..len].iter().cloned().collect()];
                               out.write_stream_f32(min_frame_count, &frames).unwrap();
                           });
                           out.register_underflow_callback(|out: rsoundio::OutStream| {
                               println!("Underflow in {} occured!", out.name().unwrap())
                           });
                           audio_init.wait();
                           out.open().unwrap();
                           match out.latency() {
                               Ok(latency) => println!("SW-latency: {}", latency),
                               Err(err) => println!("err: {}", err),
                           }
                           out.start().unwrap();
                           // Get handle of the current thread and park it.
                           // The thread will be unparked when the application quits.
                           thread::park();
                           println!("Disconnecting audio backend.");
                           sio.disconnect();
                       })
                       .unwrap());

    // Wait until EOF is received.
    try!(read_eof());
    quit.store(true, Ordering::Relaxed);
    if let Some(handle) = handles.remove("dsp") {
        handle.join().unwrap();
    }
    if let Some(handle) = handles.remove("output") {
        handle.thread().unpark();
        handle.join().unwrap();
    }
    Ok(())
}

fn read_eof() -> Result<(), RunError> {
    let mut buffer = String::new();
    let mut eof = false;
    while !eof {
        // Read from `stdin` until Ctrl-D (`EOF`) is received.
        eof = try!(::std::io::stdin()
                       .read_line(&mut buffer)
                       .map_err(|err| RunError::IoError(err))) == 0;
    }
    Ok(())
}
