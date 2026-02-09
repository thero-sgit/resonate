use std::{io::Cursor};
use rayon::{iter::ParallelIterator, slice::ParallelSlice};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, WindowFunction};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{Decoder, DecoderOptions},
    formats::{FormatOptions, FormatReader, Track},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint
};


pub fn ingest(bytes: &[u8]) -> Vec<f32> {
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

    let chunk_size = 1024;
    process(chunk_size, input, input_rate, output_rate)
}

fn process(chunk_size: usize, input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    let thread_batch_size = chunk_size * 100;

    input
        .par_chunks(thread_batch_size)
        .map(|segment| {
            let params = SincInterpolationParameters {
                sinc_len: 128,
                f_cutoff: 0.95,
                oversampling_factor: 64,
                interpolation: rubato::SincInterpolationType::Linear,
                window: WindowFunction::BlackmanHarris2,
            };

            let mut resampler = SincFixedIn::<f32>::new(
                output_rate as f64 / input_rate as f64,
                2.0,
                params,
                chunk_size.clone(),
                1
            ).unwrap();

            let mut local_output = Vec::new();
            let mut position = 0;
            
            while position + chunk_size <= segment.len() {
                let chunk = vec![segment[position.. position + chunk_size].to_vec()];

                let result = resampler.process(&chunk, None).unwrap();

                local_output.extend_from_slice(&result[0]);
                position += chunk_size;
            }

            let remaining = segment.len() - position;
            if remaining > 0 {
                let mut padded = vec![0.0; chunk_size];
                padded[..remaining].copy_from_slice(&segment[position..]);
                let result = resampler.process(&[padded], None).unwrap();
                local_output.extend_from_slice(&result[0]);
            }

            local_output

        })
        .flatten()
        .collect()
}


fn to_mono(input: &[f32], channels: usize) -> Vec<f32> {
    input
        .par_chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn decode_audio(bytes: &[u8]) -> (Vec<f32>, u32, usize) {
    let cursor = Cursor::new(bytes.to_vec());
    let media_source_stream = MediaSourceStream::new(Box::new(cursor), 
        Default::default());

    let mut format = get_format(media_source_stream);
    let track = format.default_track().unwrap();

    let mut decoder = get_decoder(track);

    let mut samples = Vec::new();
    let sample_rate = track.codec_params.sample_rate.unwrap();
    let channels = track.codec_params.channels.unwrap().count();
    let track_id = track.id;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => break,
        };

        if packet.track_id() != track_id {continue;}

        let decoded = decoder.decode(&packet);
        if decoded.is_err() {continue;}
        let decoded = decoded.unwrap();

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