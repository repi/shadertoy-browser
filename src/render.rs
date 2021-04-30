use std::any::Any;

use crate::errors::*;

#[repr(C)]
#[cfg_attr(
    feature = "wgpu-backend",
    derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)
)]
#[allow(non_snake_case)]
pub struct ShadertoyConstants {
    // The viewport resolution (z is pixel aspect ratio, usually 1.0).
    pub iResolution: [f32; 3],
    pub pad1: f32,
    /// xy contain the current pixel coords (if LMB is down). zw contain the click pixel.
    pub iMouse: [f32; 4],
    /// Current time in seconds.
    pub iTime: f32,
    /// Delta time since last frame.
    pub iTimeDelta: f32,
    /// Number of frames rendered per second.
    pub iFrameRate: f32,
    /// Sound sample rate (typically 44100).
    pub iSampleRate: f32,
    /// Current frame.
    pub iFrame: i32,
    pub pad2: [i32; 3],
    /// Year, month, day, time in seconds in .xyzw
    pub iDate: [f32; 4],
    /// Time for channel (if video or sound), in seconds.
    pub iChannelTime: [f32; 4],
    /// Input texture resolution for each channel.
    pub iChannelResolution: [[f32; 4]; 4],
    pub iBlockOffset: f32,
    pub pad3: [f32; 3],
}

pub type RenderPipelineHandle = usize;

pub struct RenderQuad {
    /// x & y position of quad in normalized [0,1] coordinates.
    pub pos: (f32, f32),
    /// width & height of quad in normalized [0,1] coordinates.
    pub size: (f32, f32),
    /// The shader pipeline to use.
    pub pipeline_handle: RenderPipelineHandle,
}

pub struct RenderParams<'a> {
    pub clear_color: (f32, f32, f32, f32),
    pub mouse_pos: (f64, f64),
    pub mouse_click_pos: (f64, f64),
    pub quads: &'a [RenderQuad],
}

pub trait RenderBackend: Sync {
    fn init_window(&mut self, window: &dyn Any);
    fn render_frame(&mut self, params: RenderParams<'_>);

    fn new_pipeline(&self, shader_path: &str, shader_source: &str) -> Result<RenderPipelineHandle>;
}
