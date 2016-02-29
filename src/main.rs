extern crate docopt;
extern crate rustc_serialize;

/// This is a raw string that does'nt need to be escaped.
const USAGE: &'static str = r#"
Ytterbium OSC controllable synthesizer

Usage:
    ytterbium [--port=<port> --addr=<addr> --sample-rate=<sr>]
    ytterbium (-h | --help | --version)

Options:
    --port=<port>       OSC listening port. [default: 9090]
    --addr=<addr>       Network interface to listen on. [default: 0.0.0.0]
    --sample-rate=<sr>  Playback sample rate. [default: 48000]
    -h --help           Show this help page.
    --version           Print the version number and exit.
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_port: usize,
    flag_addr: String,
    flag_sample_rate: usize,
    flag_help: bool,
    flag_version: bool,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let argv: Vec<String> = ::std::env::args().collect();
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|docopt| {
                             let version = Some(format!("ytterbium {}", VERSION));
                             docopt.version(version)
                                   .argv(argv.into_iter())
                                   .decode()
                         })
                         .unwrap_or_else(|err| err.exit());
    println!("{:?}", args)
}
