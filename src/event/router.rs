extern crate rosc;
extern crate portmidi;

use std::sync::mpsc;

use receiver::MidiEvent;
use event::{ControlEvent, RawControlEvent};

pub struct EventRouter<R, S: From<R>> {
    rx: mpsc::Receiver<R>,
    tx: mpsc::Sender<S>,
}
impl<R, S: From<R>> EventRouter<R, S> {
    pub fn new(rx: mpsc::Receiver<R>, tx: mpsc::Sender<S>) -> Self {
        EventRouter { tx: tx, rx: rx }
    }

    pub fn route(&self) {
        loop {
            let in_msg = self.rx.recv().unwrap();
            self.tx.send(S::from(in_msg)).unwrap();
        }
    }
}

