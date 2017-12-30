extern crate libc;
extern crate foreign_types;
extern crate metal_rs as metal;
extern crate winit;
extern crate shaderc;
extern crate spirv_cross;

extern crate objc_foundation;
extern crate cocoa;

use std::time::Instant;
use std::mem;
use std::any::Any;

use std::ffi::CStr;
use objc::runtime::{Object, YES};
use self::foreign_types::ForeignType;

use cocoa::base::id as cocoa_id;
use cocoa::foundation::NSSize;
use cocoa::appkit::{NSWindow, NSView};
use winit::os::macos::WindowExt;

use floating_duration::TimeAsFloat;
use chrono::prelude::*;

use render::*;

pub struct MetalRenderBackend {
    device: metal::Device,
    command_queue: metal::CommandQueue,

    layer: Option<metal::CoreAnimationLayer>,
    frame_index: u64,
    time: Instant,
    time_last_frame: Instant,

    shader_source: String,
    pipeline_state: Option<metal::RenderPipelineState>,
}


impl MetalRenderBackend {
    pub fn new() -> MetalRenderBackend {
        let device = metal::Device::system_default();

        let command_queue = device.new_command_queue();

        MetalRenderBackend {
            device,
            command_queue,
            layer: None,
            frame_index: 0,
            time: Instant::now(),
            time_last_frame: Instant::now(),
            shader_source: String::new(),
            pipeline_state: None,
        }
    }

    fn create_pipeline_state(&self, shader_source: String) -> Result<metal::RenderPipelineState, String> {
        let compile_options = metal::CompileOptions::new();

        let vs_source = include_str!("shadertoy_vs.metal");
        let ps_source = shader_source;

        let vs_library = new_library_with_source(&self.device, vs_source, &compile_options)?;
        let ps_library = new_library_with_source(&self.device, &ps_source, &compile_options)?;

        let vs = vs_library.get_function("vsMain", None)?;
        let ps = ps_library.get_function("main0", None)?;

        let vertex_desc = metal::VertexDescriptor::new();

        let pipeline_desc = metal::RenderPipelineDescriptor::new();
        pipeline_desc.set_vertex_function(Some(&vs));
        pipeline_desc.set_fragment_function(Some(&ps));
        pipeline_desc.set_vertex_descriptor(Some(vertex_desc));
        pipeline_desc
            .color_attachments()
            .object_at(0)
            .unwrap()
            .set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);

        //return self.device.new_render_pipeline_state(&pipeline_desc);
        return new_render_pipeline_state(&self.device, &pipeline_desc);
    }

    fn update_shader(&mut self, shader_source: String) {

        if shader_source == self.shader_source {
            return;
        }

        self.shader_source = shader_source.clone();

        self.pipeline_state = match self.create_pipeline_state(shader_source) {
            Ok(pipeline_state) => Some(pipeline_state),
            Err(string) => {
                if self.shader_source.len() > 0 {
                    println!("Error creating pipeline state: {}", string);
                }
                None
            }
        }
    }
}

impl RenderBackend for MetalRenderBackend {
    fn init_window(&mut self, window: &Any) {

        let winit_window = window.downcast_ref::<winit::Window>().unwrap();

        let cocoa_window: cocoa_id = unsafe { mem::transmute(winit_window.get_nswindow()) };

        let layer = metal::CoreAnimationLayer::new();
        layer.set_device(&self.device);
        layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);

        unsafe {
            let view = cocoa_window.contentView();
            view.setWantsBestResolutionOpenGLSurface_(YES);
            view.setWantsLayer(YES);
            view.setLayer(mem::transmute(layer.as_ref()));
        }

        let draw_size = winit_window.get_inner_size().unwrap();
        layer.set_drawable_size(NSSize::new(draw_size.0 as f64, draw_size.1 as f64));

        self.layer = Some(layer);
    }

    fn present(&mut self, params: RenderParams) {

        self.update_shader(params.shader_source);

        //println!("frame: {}", self.frame_index);

        if let Some(ref layer) = self.layer {
            if let Some(drawable) = layer.next_drawable() {

                let constants = {
                    let w = drawable.texture().width() as f32;
                    let h = drawable.texture().height() as f32;

                    let time = self.time.elapsed().as_fractional_secs() as f32;
                    let delta_time = self.time_last_frame.elapsed().as_fractional_secs() as f32;

                    let dt: DateTime<Local> = Local::now();

                    ShadertoyConstants {
                        iResolution: (w, h, w / h),
                        pad1: 0.0,
                        iMouse: (params.mouse_cursor_pos.0 as f32, params.mouse_cursor_pos.1 as f32, 0.0, 0.0),
                        iTime: time,
                        iTimeDelta: delta_time,
                        iFrameRate: 1.0 / delta_time,
                        iSampleRate: 44100.0,
                        iFrame: self.frame_index as i32,
                        pad2: [0, 0, 0],
                        iDate: (
                            dt.year() as f32,
                            dt.month() as f32,
                            dt.day() as f32,
                            dt.second() as f32, // TODO unclear what seconds should be here?
                        ),
                        iChannelTime: [time, time, time, time], // TODO not correct
                        iChannelResolution: [
                            (0.0, 0.0, 0.0, 0.0),
                            (0.0, 0.0, 0.0, 0.0),
                            (0.0, 0.0, 0.0, 0.0),
                            (0.0, 0.0, 0.0, 0.0),
                        ],
                        iBlockOffset: 0.0,
                        pad3: [0.0, 0.0, 0.0],
                    }
                };

                let render_pass_descriptor = metal::RenderPassDescriptor::new();
                let color_attachment = render_pass_descriptor
                    .color_attachments()
                    .object_at(0)
                    .unwrap();
                color_attachment.set_texture(Some(drawable.texture()));
                color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                color_attachment.set_clear_color(metal::MTLClearColor::new(1.0, 0.0, 0.0, 1.0));
                color_attachment.set_store_action(metal::MTLStoreAction::Store);

                let command_buffer = self.command_queue.new_command_buffer();
                let parallel_encoder = command_buffer.new_parallel_render_command_encoder(&render_pass_descriptor);
                let encoder = parallel_encoder.render_command_encoder();
                
                if let Some(ref pipeline_state) = self.pipeline_state {
                    encoder.set_render_pipeline_state(&pipeline_state);
                    encoder.set_cull_mode(metal::MTLCullMode::None);

                    let constants_ptr: *const ShadertoyConstants = &constants;
                    let constants_cptr = constants_ptr as *mut libc::c_void;
                    encoder.set_vertex_bytes(0, mem::size_of::<ShadertoyConstants>() as u64, constants_cptr);
                    encoder.set_fragment_bytes(0, mem::size_of::<ShadertoyConstants>() as u64, constants_cptr);
                    encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 3);
                }

                encoder.end_encoding();
                parallel_encoder.end_encoding();

                command_buffer.present_drawable(&drawable);
                command_buffer.commit();

                self.frame_index += 1;
                self.time_last_frame = Instant::now();
            }
        }
    }
}

// manually created version as the one in metal-rs will fail and return Err 
// for shaders that just have compilation warnings
// TODO should figure out how to resolve this properly and merge it back?
fn new_library_with_source(device: &metal::Device, src: &str, options: &metal::CompileOptionsRef) -> Result<metal::Library, String> {
    use cocoa::foundation::NSString as cocoa_NSString;
    use cocoa::base::nil as cocoa_nil;

    unsafe {
        let source = cocoa_NSString::alloc(cocoa_nil).init_str(src);

        let mut err: *mut ::objc::runtime::Object = ::std::ptr::null_mut();

        let library: *mut metal::MTLLibrary = {
            msg_send![*device, newLibraryWithSource:source
                                        options:options
                                        error:&mut err]
        };

        // TODO right now we just return Ok if a library is built, and ignore the warnings
        // would be ideal to be able to report out warnings even for successful builds though
        if !library.is_null() {
            return Result::Ok(metal::Library::from_ptr(library));
        }

        if !err.is_null() {
            let desc: *mut Object = msg_send![err, localizedDescription];
            let compile_error: *const libc::c_char = msg_send![desc, UTF8String];
            let message = CStr::from_ptr(compile_error).to_string_lossy().into_owned();
            // original code crashes due to this release when having error message
            //msg_send![err, release]; 
            return Err(message);
        }

        return Err(String::from("unreachable?"));
    }
}

macro_rules! try_objc {
    {
        $err_name: ident => $body:expr
    } => {
        {
            let mut $err_name: *mut ::objc::runtime::Object = ::std::ptr::null_mut();
            let value = $body;
            if !$err_name.is_null() {
                let desc: *mut Object = msg_send![$err_name, localizedDescription];
                let compile_error: *const libc::c_char = msg_send![desc, UTF8String];
                let message = CStr::from_ptr(compile_error).to_string_lossy().into_owned();
                //msg_send![$err_name, release];
                return Err(String::from("error apa"));
            }
            value
        }
    };
}

// manually created version as the one in metal-rs will return Ok even if 
// a null pipeline state is returnedfail and return Err 
// TODO should merge this back 
pub fn new_render_pipeline_state(device: &metal::Device, descriptor: &metal::RenderPipelineDescriptorRef) -> Result<metal::RenderPipelineState, String> {
    unsafe {
        let pipeline_state: *mut metal::MTLRenderPipelineState = try_objc!{ err =>
            msg_send![*device, newRenderPipelineStateWithDescriptor:descriptor
                                                            error:&mut err]
        };

        // This is the check that is new here
        // apparently there are cases where an error message is not returned but null is
        if pipeline_state.is_null() {
            return Err(String::from("newRenderPipelineStateWithDescriptor returned null"));
        }

        Ok(metal::RenderPipelineState::from_ptr(pipeline_state))
    }
}

pub fn convert_glsl_to_metal(name: &str, entry_point: &str, source: &str) -> Result<String, String> {

    // convert to SPIR-V using shaderc

    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();

    let binary_result = match compiler.compile_into_spirv(source, shaderc::ShaderKind::Fragment, name, entry_point, Some(&options)) {

        Ok(result) => result,
        Err(err) => {
            return Err(format!("shaderc compilation failed: {}", err));
        }
    };

    // convert SPIR-V to MSL

    use self::spirv_cross::{spirv, msl};

    let module = spirv::Module::from_words(binary_result.as_binary());

    let mut ast = spirv::Ast::<msl::Target>::parse(&module).unwrap();

    match ast.compile() {
        Ok(str) => Ok(str),
        Err(e) => {
            match e {
                spirv_cross::ErrorCode::Unhandled => Err(String::from("spirv-cross handled error")),
                spirv_cross::ErrorCode::CompilationError(str) => Err(format!("spirv-cross error: {}", str)),
            }
        }
    }
}
