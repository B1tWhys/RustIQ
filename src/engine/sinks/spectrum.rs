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
}

impl Block for SpectrumSink {
    fn work(&mut self) -> Result<BlockRet, Error> {
        let (input, _tags) = self.src.read_buf()?;

        if input.is_empty() {
            return Ok(BlockRet::Noop);
        }

        let n = input.len();

        // Convert to owned Vec and send via channel
        let mut spectrum_data = Vec::with_capacity(size_of::<f32>() * n);
        input.iter().for_each(|i: &f32| spectrum_data.push(*i));
        
        // Use try_send to avoid blocking the DSP pipeline
        match self.event_tx.try_send(Event::SpectrumData(spectrum_data)) {
            Ok(_) => {},
            Err(flume::TrySendError::Full(_)) => {
                // Channel full - UI is busy
                eprintln!("Warning: Event channel full, dropping spectrum frame");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                // UI has disconnected, terminate the engine
                return Err(Error::new("Event channel disconnected"));
            }
        }

        // Consume all input samples
        input.consume(n);

        Ok(BlockRet::Ok)
    }
}
