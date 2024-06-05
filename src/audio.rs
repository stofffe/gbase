use crate::Context;
use rodio::{source::Buffered, Decoder, Source};
use std::{io::Cursor, path::PathBuf};

pub type AudioHandle = usize;
pub type SoundSource = Buffered<Decoder<Cursor<Vec<u8>>>>;

pub(crate) struct AudioContext {
    raw_handle: AudioHandleRaw,

    sources: Vec<SoundSource>,
}

struct AudioHandleRaw {
    handle: rodio::OutputStreamHandle,
    _stream: rodio::OutputStream, // store handle to keep output stream alive
}

impl AudioHandleRaw {
    fn new() -> Self {
        let (_stream, handle) =
            rodio::OutputStream::try_default().expect("could not initalize output stream");
        Self { handle, _stream }
    }
}

impl AudioContext {
    pub(crate) fn new() -> Self {
        let raw_handle = AudioHandleRaw::new();
        let sources = Vec::new();
        Self {
            raw_handle,
            sources,
        }
    }

    fn load_audio_source(&mut self, bytes: Vec<u8>) -> AudioHandle {
        let source = rodio::Decoder::new(Cursor::new(bytes))
            .expect("could not decode audio")
            .buffered();
        self.sources.push(source);
        self.sources.len() - 1
    }

    fn play_sound(&self, handle: AudioHandle) {
        let source = self.sources[handle].clone();

        // TODO handle error
        self.raw_handle
            .handle
            .play_raw(source.convert_samples())
            .expect("could not play sound");
    }
}

//
// Commands
//

// Load audio source
pub async fn load_audio_source(ctx: &mut Context, path: impl Into<PathBuf>) -> AudioHandle {
    let bytes = ctx
        .filesystem
        .load_bytes(&path.into())
        .await
        .expect("could not load bytes");
    ctx.audio.load_audio_source(bytes)
}

/// Play audio source as raw sound
pub fn play_audio_source(ctx: &Context, handle: AudioHandle) {
    ctx.audio.play_sound(handle);
}
