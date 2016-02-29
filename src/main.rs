extern crate docopt;
extern crate rustc_serialize;

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
    println!("{:?}", args)
}
