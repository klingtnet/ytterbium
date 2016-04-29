#[macro_use]
extern crate approx;

extern crate rosc;
extern crate rsoundio;
extern crate rb;

extern crate clap;

use std::cmp;
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::sync::atomic::{AtomicBool, Ordering};
use std::str::FromStr;
use std::net::{IpAddr, SocketAddr};
use std::process;
use std::io::Write;

use rb::{RB, RbProducer, RbConsumer};

mod event;
mod io;
mod dsp;
mod types;
mod errors;
use io::{Receiver, OscReceiver, MidiReceiver};
use event::Controllable;
use types::*;
use errors::RunError;

const MAX_VOICES: usize = 24;
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

macro_rules! printerr(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("could not write to stderr");
    } }
);


struct Args {
    socket_addr_in: SocketAddr,
    socket_addr_out: SocketAddr,
    sample_rate: usize,
}

/// Parses and validates the command line arguments.
/// If an error occurs a message is written to stderr and
/// the program exits.
fn get_args() -> Args {
    let address_arg = clap::Arg::with_name("address")
                          .long("address")
                          .short("a")
                          .required(true)
                          .takes_value(true)
                          .value_name("ip-address")
                          .default_value("0.0.0.0")
                          .help("Address to listen on for OSC messages.");
    let ports_arg = clap::Arg::with_name("ports")
                        .long("ports")
                        .short("p")
                        .required(true)
                        .takes_value(true)
                        .number_of_values(2)
                        .value_names(&["in", "out"])
                        .help("OSC listening and send port.");
    let sample_rate_arg = clap::Arg::with_name("sample-rate")
                              .long("sample-rate")
                              .short("s")
                              .takes_value(true)
                              .value_name("sample-rate")
                              .default_value("48000")
                              .possible_values(&["44100", "48000", "88200", "96000"])
                              .help("Playback sample-rate");
    let args = clap::App::new("ytterbium")
                   .version(VERSION)
                   .author("Andreas Linz <klingt.net@gmail.com>")
                   .arg(address_arg)
                   .arg(ports_arg)
                   .arg(sample_rate_arg)
                   .get_matches();

    let sample_rate = args.value_of("sample-rate")
                          .map_or(48_000, |str_val| str_val.parse::<usize>().unwrap());
    let ip_addr = match IpAddr::from_str(args.value_of("address").unwrap()) {
        Ok(val) => val,
        Err(err) => {
            printerr!("Bad ip address: {}", err);
            process::exit(1)
        }
    };
    let ports = args.values_of("ports")
                    .unwrap()
                    .map(|port| {
                        match port.parse::<u16>() {
                            Ok(val) => val,
                            Err(err) => {
                                printerr!("Bad port, must be in range [0, 65535]: {}", err);
                                process::exit(1)
                            }
                        }
                    })
                    .collect::<Vec<u16>>();
    let (socket_addr_in, socket_addr_out) = (SocketAddr::new(ip_addr, ports[0]),
                                             SocketAddr::new(ip_addr, ports[1]));

    Args {
        socket_addr_in: socket_addr_in,
        socket_addr_out: socket_addr_out,
        sample_rate: sample_rate,
    }
}

fn main() {
    let args = get_args();
    run(args)
        .map_err(|err| {
            printerr!("{:?}", err);
            process::exit(1)
        })
        .unwrap();
}

fn run(args: Args) -> Result<(), RunError> {
    let buf = rb::SpscRb::new(4096);
    let (producer, consumer) = (buf.producer(), buf.consumer());
    let (tx_receiver, rx_dsp) = mpsc::channel();
    let audio_init = Arc::new(Barrier::new(1));
    let mut handles = HashMap::with_capacity(5);
    let quit = Arc::new(AtomicBool::new(false));

    handles.insert("osc",
                   thread::Builder::new()
                       .name("osc".to_owned())
                       .spawn({
                           let tx = tx_receiver.clone();
                           let socket_addr = args.socket_addr_in;
                           move || {
                               let mut osc_receiver = OscReceiver::new(socket_addr).unwrap();
                               osc_receiver.receive_and_send(tx)
                           }
                       })
                       .unwrap());

    handles.insert("midi",
                   thread::Builder::new()
                       .name("midi".to_owned())
                       .spawn({
                           let tx = tx_receiver.clone();
                           move || {
                               let mut midi_receiver = MidiReceiver::new().unwrap();
                               midi_receiver.receive_and_send(tx)
                           }
                       })
                       .unwrap());

    handles.insert("dsp",
                   thread::Builder::new()
                       .name("dsp".to_owned())
                       .spawn({
                           let init = audio_init.clone();
                           let quit = quit.clone();
                           let sample_rate = args.sample_rate;
                           move || {
                               let mut adsr = dsp::ADSR::new(sample_rate);
                               let mut osc = dsp::WavetableOsc::new(sample_rate);
                               let mut buf: [Float; 32] = [0.0; 32];

                               init.wait();
                               loop {
                                   if quit.load(Ordering::Relaxed) {
                                       break;
                                   }
                                   for sample in &mut buf {
                                       if let Ok(msg) = rx_dsp.try_recv() {
                                           adsr.handle(&msg);
                                           osc.handle(&msg);
                                       }
                                       *sample = osc.tick() * adsr.tick();
                                   }
                                   producer.write_blocking(&buf).unwrap();
                               }
                           }
                       })
                       .unwrap());

    handles.insert("output",
                   thread::Builder::new()
                       .name("output".to_owned())
                       .spawn({
                           let init = audio_init.clone();
                           let sample_rate = args.sample_rate as u32;
                           move || {
                                let mut sio = rsoundio::SoundIo::new();
                                sio.set_name("ytterbium").unwrap();
                                // connect to default backend
                                sio.connect().unwrap();
                                sio.flush_events();
                                let dev = sio.default_output_device().unwrap();
                                let mut out_stream = dev.create_outstream().unwrap();
                                out_stream.set_name("ytterbium").ok();
                                out_stream.set_format(rsoundio::SioFormat::Float32LE).unwrap();
                                out_stream.set_sample_rate(sample_rate);

                                init.wait();
                                out_stream.register_write_callback(|out: rsoundio::OutStream,
                                                                  min_frame_count: u32,
                                                                  max_frame_count: u32| {
                                    const LEN: usize = 2048;
                                    // TODO: use a length that is not smaller than 2048 for pulseaudio
                                    let len = cmp::min(LEN, min_frame_count as usize);
                                    let mut data: Vec<Float> = vec![0.0; LEN];
                                    consumer.read_blocking(&mut data[..len]).unwrap();
                                    let frames = vec![data[..len].iter().map(|s| *s as f32).collect(),
                                                      data[..len].iter().map(|s| *s as f32).collect()];
                                    match out.write_stream_f32(min_frame_count, &frames) {
                                        Ok(_) => (),
                                        Err(err) => println!("{}", err),
                                    }
                                });

                                out_stream.register_underflow_callback(|out: rsoundio::OutStream| {
                                    println!("Underflow in {} occured!", out.name().unwrap())
                                });
                                out_stream.open().unwrap();
                                match out_stream.latency() {
                                    Ok(latency) => println!("SW-latency: {:.2} ms", 1000.0*latency),
                                    Err(err) => println!("err: {}", err),
                                }
                                out_stream.start().unwrap();
                                // Get handle of the current thread and park it.
                                // The thread will be unparked when the application quits.
                                thread::park();
                                sio.disconnect();
                           }
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
                       .map_err(RunError::IoError)) == 0;
    }
    Ok(())
}
