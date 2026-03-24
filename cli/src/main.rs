use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use stadium_eq_core::config::PipelineConfig;
use stadium_eq_core::pipeline::Pipeline;
use stadium_eq_core::separation::mix_controller::UserMix;

// ---------------------------------------------------------------------------
// CLI argument definitions
// ---------------------------------------------------------------------------

/// Stadium audio equalizer – CLI interface.
#[derive(Parser, Debug)]
#[command(name = "stadium-eq-cli", version, about)]
struct Cli {
    /// Input WAV file path (omit for real-time mode).
    input: Option<String>,

    /// Output WAV file path.
    #[arg(default_value = "output.wav")]
    output: Option<String>,

    /// Sample rate in Hz.
    #[arg(long = "sample-rate", default_value_t = 48000)]
    sample_rate: u32,

    /// Frame size in samples.
    #[arg(long = "frame-size", default_value_t = 480)]
    frame_size: usize,

    /// FFT size.
    #[arg(long = "fft-size", default_value_t = 1024)]
    fft_size: usize,

    /// Calibration duration in seconds.
    #[arg(long = "calibrate-seconds", default_value_t = 7)]
    calibrate_seconds: u32,

    /// Crowd mix level (-1.0 to 1.0).
    #[arg(long = "crowd", default_value_t = 0.0)]
    crowd: f32,

    /// Speaker mix level (-1.0 to 1.0).
    #[arg(long = "speaker", default_value_t = 0.0)]
    speaker: f32,

    /// Music mix level (-1.0 to 1.0).
    #[arg(long = "music", default_value_t = 0.0)]
    music: f32,

    /// Overall gain in dB.
    #[arg(long = "gain", default_value_t = 0.0)]
    gain: f32,

    /// Bypass processing (passthrough).
    #[arg(long = "bypass")]
    bypass: bool,

    /// Skip calibration phase.
    #[arg(long = "no-calibrate")]
    no_calibrate: bool,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.input {
        Some(ref _input_path) => run_file_mode(&cli),
        None => run_realtime_mode(&cli),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `PipelineConfig` from CLI arguments.
fn build_config(cli: &Cli) -> PipelineConfig {
    PipelineConfig {
        sample_rate: cli.sample_rate,
        frame_size: cli.frame_size,
        fft_size: cli.fft_size,
        hop_size: cli.frame_size,
        calibration_duration_ms: cli.calibrate_seconds * 1000,
        ..PipelineConfig::default()
    }
}

/// Build a `UserMix` from CLI arguments.
fn build_mix(cli: &Cli) -> UserMix {
    UserMix {
        crowd_level: cli.crowd,
        speaker_level: cli.speaker,
        music_level: cli.music,
        overall_gain_db: cli.gain,
    }
}

// ---------------------------------------------------------------------------
// File processing mode
// ---------------------------------------------------------------------------

fn run_file_mode(cli: &Cli) -> Result<()> {
    let input_path = cli.input.as_ref().unwrap();
    let output_path = cli.output.as_deref().unwrap_or("output.wav");

    // 1. Read input WAV.
    println!("Reading input: {}", input_path);
    let mut reader = hound::WavReader::open(input_path)
        .with_context(|| format!("Failed to open input WAV: {}", input_path))?;

    let spec = reader.spec();
    println!(
        "  Format: {} ch, {} Hz, {:?} {}-bit",
        spec.channels, spec.sample_rate, spec.sample_format, spec.bits_per_sample
    );

    // Convert all samples to f32.
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<f32>, _>>()
            .context("Failed to read float samples")?,
        hound::SampleFormat::Int => {
            let max_val = (1u32 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .collect::<std::result::Result<Vec<i32>, _>>()
                .context("Failed to read int samples")?
                .into_iter()
                .map(|s| s as f32 / max_val)
                .collect()
        }
    };

    // If stereo (or more), mix down to mono.
    let mono_samples: Vec<f32> = if spec.channels > 1 {
        let ch = spec.channels as usize;
        println!("  Mixing {} channels down to mono", ch);
        samples
            .chunks(ch)
            .map(|frame| frame.iter().sum::<f32>() / ch as f32)
            .collect()
    } else {
        samples
    };

    let total_samples = mono_samples.len();
    println!("  Total mono samples: {}", total_samples);

    // 2. Create pipeline.
    let config = build_config(cli);
    let mut pipeline = Pipeline::new(config);

    // 3. Start calibration if requested.
    if !cli.no_calibrate {
        println!(
            "Starting calibration ({} seconds worth of audio)...",
            cli.calibrate_seconds
        );
        pipeline.start_calibration();
    }

    // 4. Set user mix.
    pipeline.set_mix(build_mix(cli));

    // 5. Set bypass if requested.
    if cli.bypass {
        pipeline.set_bypass(true);
        println!("Bypass mode enabled.");
    }

    // 6. Process frame by frame.
    let frame_size = cli.frame_size;
    let mut output_samples: Vec<f32> = Vec::with_capacity(total_samples);
    let mut frames_processed: usize = 0;
    let mut frame_output: Vec<f32> = Vec::new();

    let mut offset = 0;
    while offset < mono_samples.len() {
        let end = (offset + frame_size).min(mono_samples.len());
        let chunk = &mono_samples[offset..end];

        // If the last chunk is shorter than frame_size, zero-pad it.
        let input_frame: Vec<f32> = if chunk.len() < frame_size {
            let mut padded = vec![0.0f32; frame_size];
            padded[..chunk.len()].copy_from_slice(chunk);
            padded
        } else {
            chunk.to_vec()
        };

        pipeline.process_frame(&input_frame, &mut frame_output);

        // Only keep as many output samples as we had real input (trim last frame).
        let take = end - offset;
        output_samples.extend_from_slice(&frame_output[..take.min(frame_output.len())]);
        frames_processed += 1;

        offset = end;
    }

    // 7. Write output WAV.
    println!("Writing output: {}", output_path);
    let out_spec = hound::WavSpec {
        channels: 1,
        sample_rate: cli.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(output_path, out_spec)
        .with_context(|| format!("Failed to create output WAV: {}", output_path))?;

    for &sample in &output_samples {
        writer.write_sample(sample)?;
    }
    writer.finalize().context("Failed to finalize output WAV")?;

    // 8. Print stats.
    println!("--- Processing complete ---");
    println!("  Frames processed: {}", frames_processed);
    println!(
        "  Output samples:   {} ({:.2}s at {} Hz)",
        output_samples.len(),
        output_samples.len() as f64 / cli.sample_rate as f64,
        cli.sample_rate
    );

    let state = pipeline.get_state();
    println!("  Pipeline state:   {:?}", state);

    if let Some(cal) = pipeline.get_calibration_result() {
        println!("  Calibration result:");
        println!("    Environment:       {:?}", cal.environment);
        println!("    Crowd density:     {:?}", cal.crowd_density);
        println!(
            "    Dominant freq:     {:.0}-{:.0} Hz",
            cal.dominant_frequency_range.0, cal.dominant_frequency_range.1
        );
        println!("    Estimated SNR:     {:.1} dB", cal.estimated_snr.0);
    } else if !cli.no_calibrate {
        println!("  Calibration: did not complete (input may be too short)");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Real-time processing mode
// ---------------------------------------------------------------------------

fn run_realtime_mode(cli: &Cli) -> Result<()> {
    println!("Real-time mode: connect microphone");
    println!("  Sample rate: {} Hz", cli.sample_rate);
    println!("  Frame size:  {} samples", cli.frame_size);
    println!("  Press Ctrl+C to stop.\n");

    // Build pipeline.
    let config = build_config(cli);
    let pipeline = Rc::new(RefCell::new(Pipeline::new(config)));

    // Apply user settings.
    {
        let mut pl = pipeline.borrow_mut();
        pl.set_mix(build_mix(cli));

        if cli.bypass {
            pl.set_bypass(true);
        } else if !cli.no_calibrate {
            pl.start_calibration();
        }
    }

    // Shared ring buffers: input callback -> processing thread -> output callback.
    let input_ring: Arc<Mutex<VecDeque<f32>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(cli.frame_size * 16)));
    let output_ring: Arc<Mutex<VecDeque<f32>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(cli.frame_size * 16)));

    // --- Discover audio devices ---
    let host = cpal::default_host();

    let input_device = host
        .default_input_device()
        .context("No input audio device available")?;
    println!("Input device:  {:?}", input_device.name()?);

    let output_device = host
        .default_output_device()
        .context("No output audio device available")?;
    println!("Output device: {:?}", output_device.name()?);

    let stream_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(cli.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    // --- Input stream ---
    let input_ring_writer = Arc::clone(&input_ring);
    let input_stream = input_device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut ring) = input_ring_writer.lock() {
                    ring.extend(data.iter());
                }
            },
            |err| eprintln!("Input stream error: {}", err),
            None,
        )
        .context("Failed to build input stream")?;

    // --- Output stream ---
    let output_ring_reader = Arc::clone(&output_ring);
    let output_stream = output_device
        .build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut ring) = output_ring_reader.lock() {
                    for sample in data.iter_mut() {
                        *sample = ring.pop_front().unwrap_or(0.0);
                    }
                } else {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                }
            },
            |err| eprintln!("Output stream error: {}", err),
            None,
        )
        .context("Failed to build output stream")?;

    // Start streams.
    input_stream
        .play()
        .context("Failed to start input stream")?;
    output_stream
        .play()
        .context("Failed to start output stream")?;
    println!("Streams started. Processing audio...\n");

    // Ctrl+C / stop flag.
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = Arc::clone(&running);
        std::thread::spawn(move || {
            let mut buf = String::new();
            println!("Press Enter to stop...");
            let _ = std::io::stdin().read_line(&mut buf);
            r.store(false, Ordering::SeqCst);
        });
    }

    // Processing loop: drain input ring, process frames, push to output ring.
    let frame_size = cli.frame_size;
    let mut frame_buf = vec![0.0f32; frame_size];
    let mut frame_output = Vec::new();

    while running.load(Ordering::SeqCst) {
        // Check if we have a full frame available.
        let frame_ready = {
            let ring = input_ring.lock().unwrap();
            ring.len() >= frame_size
        };

        if frame_ready {
            // Drain one frame from the input ring.
            {
                let mut ring = input_ring.lock().unwrap();
                for s in frame_buf.iter_mut() {
                    *s = ring.pop_front().unwrap_or(0.0);
                }
            }

            // Process through the pipeline.
            {
                let mut pl = pipeline.borrow_mut();
                pl.process_frame(&frame_buf, &mut frame_output);
            }

            // Push processed samples to the output ring.
            {
                let mut ring = output_ring.lock().unwrap();
                ring.extend(frame_output.iter());
            }
        } else {
            // Avoid busy-spinning when no data is available.
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    println!("\nStopping streams...");
    drop(input_stream);
    drop(output_stream);

    let state = *pipeline.borrow().get_state();
    println!("Final pipeline state: {:?}", state);
    println!("Done.");

    Ok(())
}
