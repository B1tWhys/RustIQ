use flume::Sender;
use rustradio::block::{Block, BlockRet};
use rustradio::stream::ReadStream;
use rustradio::{Error, rustradio_macros};

use crate::messages::Event;

/// A sink block that consumes f32 spectrum data and sends it via flume channel.
#[derive(rustradio_macros::Block)]
#[rustradio(new)]
pub struct SpectrumSink {
    #[rustradio(in)]
    src: ReadStream<f32>,
    event_tx: Sender<Event>,
    fft_size: usize,
}

impl Block for SpectrumSink {
    fn work(&mut self) -> Result<BlockRet<'_>, Error> {
        let (input, _tags) = self.src.read_buf()?;

        // Wait until we have at least one FFT frame
        if input.len() < self.fft_size {
            return Ok(BlockRet::Pending);
        }

        // Only process one FFT frame at a time
        let n = self.fft_size;

        // Convert to owned Vec and send via channel
        let spectrum_data: Vec<f32> = input.iter().take(n).copied().collect();

        // Use try_send to avoid blocking the DSP pipeline
        match self.event_tx.try_send(Event::SpectrumData(spectrum_data)) {
            Ok(_) => {}
            Err(flume::TrySendError::Full(_)) => {
                // Channel full - UI is busy
                eprintln!("Warning: Event channel full, dropping spectrum frame");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                // UI has disconnected, terminate the engine
                return Ok(BlockRet::EOF);
            }
        }

        // Consume the FFT frame
        input.consume(n);

        Ok(BlockRet::Again)
    }
}
