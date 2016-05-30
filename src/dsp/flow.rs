use types::{SharedMut, Stereo, Wrap};

use event::{Controllable, ControlEvent};
use rb::{RbProducer, Producer};
use dsp::{SignalSource, SignalLink, SignalSink, SignalFlow, VoiceManager, SoftLimiter};

pub struct Flow {
    source: VoiceManager,
    links: Vec<SharedMut<SignalLink>>,
    sink: BufferSink,
}
impl Flow {
    pub fn new(source: VoiceManager, sink: BufferSink) -> Self {
        Flow {
            source: source,
            links: vec![SharedMut::wrap(SoftLimiter {})],
            sink: sink,
        }
    }
}
impl Controllable for Flow {
    fn handle(&mut self, msg: &ControlEvent) {
        match *msg {
            _ => {
                self.source.handle(msg);
                // TODO: send to all links
            }
        }
    }
}
impl SignalFlow for Flow {
    fn tick(&mut self) {
        let mut sample = self.source.tick();
        for link in &self.links {
            sample = link.borrow_mut().tick(sample);
        }
        self.sink.tick(sample);
    }
}

pub struct IdentityLink {}
impl SignalLink for IdentityLink {
    fn tick(&mut self, input: Stereo) -> Stereo {
        input
    }
}

pub struct BufferSink {
    position: usize,
    buffer: Vec<Stereo>,
    ring_buffer: Producer<Stereo>,
}
impl BufferSink {
    pub fn new(ring_buffer: Producer<Stereo>, chunk_size: usize) -> Self {
        BufferSink {
            position: 0,
            buffer: vec![Stereo::default(); chunk_size],
            ring_buffer: ring_buffer,
        }
    }
}
impl SignalSink for BufferSink {
    fn tick(&mut self, input: Stereo) {
        self.buffer[self.position] = input;
        if self.position == self.buffer.len() - 1 {
            self.ring_buffer.write_blocking(&self.buffer[..]).unwrap();
        }
        self.position = (self.position + 1) % self.buffer.len();
    }
}
