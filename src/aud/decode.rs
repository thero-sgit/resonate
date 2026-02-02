use std::{io::Cursor};
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
    println!("bytes: {}", &bytes.len());
    let (samples, rate, channels) = decode_audio(bytes);

    println!("samples: {}", &samples.len());
    println!("channels: {}", &channels);
    println!("rate: {}", &rate);

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
        sinc_len: 128,
        f_cutoff: 0.95,
        oversampling_factor: 64,
        interpolation: rubato::SincInterpolationType::Linear,
        window: WindowFunction::BlackmanHarris2,
    };

    let chunk_size = 1024;

    let mut resampler = SincFixedIn::<f32>::new(
        output_rate as f64 / input_rate as f64,
        2.0,
        params,
        chunk_size.clone(),
        1
    ).unwrap();

    process(chunk_size, input, &mut resampler)
}

fn process(chunk_size: usize, input: &[f32], resampler: &mut SincFixedIn<f32>) -> Vec<f32> {
    let mut output = Vec::new();
    let mut position = 0;

    while position + chunk_size <= input.len() {
        let chunk = vec![input[position.. position+chunk_size].to_vec()];
        let result = resampler.process(&chunk, None).unwrap();

        output.extend_from_slice(&result[0]);
        position += chunk_size
    }

    let remaining = input.len() - position;
    if remaining > 0 {
        let mut padded = vec![0.0; chunk_size];

        padded[..remaining].copy_from_slice(&input[position..]);

        let result = resampler.process(&[padded], None).unwrap();

        output.extend_from_slice(&result[0]);

    }

    output
}



fn to_mono(input: &[f32], channels: usize) -> Vec<f32> {
    input
        .chunks(channels)
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
        if Result::is_err(&decoded) {continue;}
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