use std::{io::Cursor};
use dasp::frame::Channels;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, WindowFunction};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{Decoder, DecoderOptions},
    formats::{FormatOptions, FormatReader, Track},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint
};

pub fn ingest(bytes: &'static [u8]) -> Vec<f32> {
    let (samples, rate, channels) = decode_audio(bytes);

    resample(
        &to_mono(&samples, channels),
        rate,
        11_025
    )
}

fn resample(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    if input_rate == output_rate {
        return input.to_vec();
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        oversampling_factor: 256,
        interpolation: rubato::SincInterpolationType::Linear,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        output_rate as f64 / input_rate as f64,
        2.0,
        params,
        1024,
        1
    ).unwrap();

    resampler.process(&[input.to_vec()], None).unwrap()[0].clone()
}

fn to_mono(input: &[f32], channels: usize) -> Vec<f32> {
    input
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn decode_audio(bytes: &'static [u8]) -> (Vec<f32>, u32, usize) {
    let cursor = Cursor::new(bytes);
    let media_source_stream = MediaSourceStream::new(Box::new(cursor), 
        Default::default());

    let mut format = get_format(media_source_stream);
    let track = format.default_track().unwrap();

    let mut decoder = get_decoder(track);

    let mut samples = Vec::new();
    let sample_rate = track.codec_params.sample_rate.unwrap();
    let channels = track.codec_params.channels.unwrap().count();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => break,
        };

        let decoded = decoder.decode(&packet).unwrap();
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());

        buffer.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buffer.samples());
    }

    (samples, sample_rate, channels)

}

fn get_format(media_source_stream: MediaSourceStream) -> Box<dyn FormatReader> {
    let hint = Hint::new();
    let probed = symphonia::default::get_probe()
        .format(&hint, media_source_stream, &FormatOptions::default(), &MetadataOptions::default())
        .expect("Unsupported format");

    probed.format
}

fn get_decoder(track: &Track) -> Box<dyn Decoder> {
    symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .unwrap()
}