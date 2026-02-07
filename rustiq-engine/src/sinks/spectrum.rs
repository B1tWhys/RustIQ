use flume::Sender;
use rustradio::block::{Block, BlockRet};
use rustradio::stream::ReadStream;
use rustradio::{Error, rustradio_macros};

use rustiq_messages::Event;

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
        // if self.src.eof() {
        //     return Ok(BlockRet::EOF);
        // }

        let (input, _tags) = self.src.read_buf()?;

        // Wait until we have at least one FFT frame
        if input.len() < self.fft_size {
            return Ok(BlockRet::Pending);
        }

        // Only process one FFT frame at a time
        let n = self.fft_size;

        // Convert to owned Vec and apply FFT shift
        let mut spectrum_data: Vec<f32> = input.iter().take(n).copied().collect();

        // FFT shift: move DC from edges to center
        // This rearranges [DC, positive, negative] -> [negative, DC, positive]
        spectrum_data.rotate_left(n / 2);

        // Block the pipeline to provide backpressure if the UI is behind
        if self
            .event_tx
            .send(Event::SpectrumData(spectrum_data))
            .is_err()
        {
            return Ok(BlockRet::EOF);
        }

        // Consume the FFT frame
        input.consume(n);

        Ok(BlockRet::Again)
    }
}
