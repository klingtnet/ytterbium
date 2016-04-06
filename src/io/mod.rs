mod midi;
mod osc;

use std::sync::mpsc;
use event::ControlEvent;

pub use self::midi::*;
pub use self::osc::*;

pub trait Receiver {
    fn receive_and_send(&mut self, mpsc::Sender<ControlEvent>);
}
