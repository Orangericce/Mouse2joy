use rodio::{OutputStream, Sink, source::SineWave};
use rodio::source::Source;
use std::time::Duration;

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (_stream, stream_handle) = OutputStream::try_default().expect("Failed to get audio output");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create sink");
        AudioPlayer { _stream, sink }
    }

    /// Play a short tone. `freq` in Hz, `ms` duration.
    pub fn play_tone(&self, freq: f32, ms: u64) {
        let source = SineWave::new(freq).take_duration(Duration::from_millis(ms)).amplify(0.25);
        self.sink.append(source);
    }

    /// Convenience wrappers for grab / release sounds.
    pub fn play_grab(&self) {
        self.play_tone(800.0, 150);
    }

    pub fn play_release(&self) {
        self.play_tone(400.0, 150);
    }
}