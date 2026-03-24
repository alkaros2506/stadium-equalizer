use stadium_eq_core::config::PipelineConfig;
use stadium_eq_core::pipeline::Pipeline;
use stadium_eq_core::separation::mix_controller::UserMix;

use std::alloc::{alloc, dealloc, Layout};

/// Initialize a new pipeline and return an owning pointer.
///
/// The caller is responsible for freeing via `stadium_eq_free`.
#[no_mangle]
pub extern "C" fn stadium_eq_init(sample_rate: u32, frame_size: u32) -> *mut Pipeline {
    let config = PipelineConfig {
        sample_rate,
        frame_size: frame_size as usize,
        hop_size: frame_size as usize,
        ..PipelineConfig::default()
    };
    let pipeline = Pipeline::new(config);
    Box::into_raw(Box::new(pipeline))
}

/// Process one frame of audio.
///
/// Reads `len` f32 samples from `input`, processes through the pipeline,
/// and writes the result to `output`. Returns the number of samples written.
#[no_mangle]
pub extern "C" fn stadium_eq_process(
    ctx: *mut Pipeline,
    input: *const f32,
    output: *mut f32,
    len: u32,
) -> u32 {
    if ctx.is_null() || input.is_null() || output.is_null() || len == 0 {
        return 0;
    }

    let pipeline = unsafe { &mut *ctx };
    let input_slice = unsafe { std::slice::from_raw_parts(input, len as usize) };

    let mut result = Vec::new();
    pipeline.process_frame(input_slice, &mut result);

    let out_len = result.len().min(len as usize);
    let output_slice = unsafe { std::slice::from_raw_parts_mut(output, out_len) };
    output_slice.copy_from_slice(&result[..out_len]);

    out_len as u32
}

/// Start the calibration phase.
#[no_mangle]
pub extern "C" fn stadium_eq_start_calibration(ctx: *mut Pipeline) {
    if ctx.is_null() {
        return;
    }
    let pipeline = unsafe { &mut *ctx };
    pipeline.start_calibration();
}

/// Update the user mix levels.
///
/// `crowd`, `speaker`, `music` are in the range -1.0 to 1.0.
/// `gain_db` is the overall gain in decibels.
#[no_mangle]
pub extern "C" fn stadium_eq_set_mix(
    ctx: *mut Pipeline,
    crowd: f32,
    speaker: f32,
    music: f32,
    gain_db: f32,
) {
    if ctx.is_null() {
        return;
    }
    let pipeline = unsafe { &mut *ctx };
    pipeline.set_mix(UserMix {
        crowd_level: crowd,
        speaker_level: speaker,
        music_level: music,
        overall_gain_db: gain_db,
    });
}

/// Enable or disable bypass mode. Pass 1 for bypass, 0 for normal.
#[no_mangle]
pub extern "C" fn stadium_eq_set_bypass(ctx: *mut Pipeline, bypass: u32) {
    if ctx.is_null() {
        return;
    }
    let pipeline = unsafe { &mut *ctx };
    pipeline.set_bypass(bypass != 0);
}

/// Free a pipeline previously created with `stadium_eq_init`.
#[no_mangle]
pub extern "C" fn stadium_eq_free(ctx: *mut Pipeline) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ctx));
    }
}

/// Allocate `size` f32 elements in WASM linear memory.
///
/// Returns a pointer the JS side can use to write audio data.
#[no_mangle]
pub extern "C" fn stadium_eq_alloc(size: u32) -> *mut f32 {
    let byte_size = (size as usize) * std::mem::size_of::<f32>();
    let align = std::mem::align_of::<f32>();
    let layout = Layout::from_size_align(byte_size, align).expect("invalid layout");
    unsafe { alloc(layout) as *mut f32 }
}

/// Deallocate memory previously allocated with `stadium_eq_alloc`.
#[no_mangle]
pub extern "C" fn stadium_eq_dealloc(ptr: *mut f32, size: u32) {
    if ptr.is_null() {
        return;
    }
    let byte_size = (size as usize) * std::mem::size_of::<f32>();
    let align = std::mem::align_of::<f32>();
    let layout = Layout::from_size_align(byte_size, align).expect("invalid layout");
    unsafe {
        dealloc(ptr as *mut u8, layout);
    }
}
