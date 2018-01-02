use std::any::Any;

use errors::*;

#[allow(non_snake_case)]
pub struct ShadertoyConstants {
    // The viewport resolution (z is pixel aspect ratio, usually 1.0).
    pub iResolution: (f32, f32, f32),
    pub pad1: f32,
    /// xy contain the current pixel coords (if LMB is down). zw contain the click pixel.
    pub iMouse: (f32, f32, f32, f32),
    /// Current time in seconds.
    pub iTime: f32,
    /// Delta time since last frame.
    pub iTimeDelta: f32,
    /// Number of frames rendered per second.
    pub iFrameRate: f32,
    /// Sound sample rate (typically 44100).
    pub iSampleRate: f32,
    /// Current frame
    pub iFrame: i32,
    pub pad2: [i32; 3],
    /// Year, month, day, time in seconds in .xyzw
    pub iDate: (f32, f32, f32, f32),
    /// Time for channel (if video or sound), in seconds
    pub iChannelTime: [f32; 4],
    /// Input texture resolution for each channel
    pub iChannelResolution: [(f32, f32, f32, f32); 4],
    pub iBlockOffset: f32,
    pub pad3: [f32; 3],
}

pub type RenderPipelineHandle = usize;

pub struct RenderParams {
    pub mouse_cursor_pos: (f64, f64),
    pub pipeline: Option<RenderPipelineHandle>,
}

pub trait RenderBackend {

    fn init_window(&mut self, window: &Any);
    fn present(&mut self, params: RenderParams);

    fn new_pipeline(&mut self, shader_source: &str) -> Result<RenderPipelineHandle>;
}
