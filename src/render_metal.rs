extern crate foreign_types;
extern crate libc;
extern crate metal_rs as metal;
extern crate shaderc;
extern crate spirv_cross;
extern crate winit;

extern crate cocoa;
extern crate objc_foundation;

use std::time::Instant;
use std::mem;
use std::any::Any;

use std::ffi::CStr;
use std::cell::RefCell;
use std::sync::Mutex;
use objc::runtime::{Object, YES};
use self::foreign_types::ForeignType;

use cocoa::base::id as cocoa_id;
use cocoa::foundation::NSSize;
use cocoa::appkit::{NSView, NSWindow};
use winit::os::macos::WindowExt;

use floating_duration::TimeAsFloat;
use chrono::prelude::*;

use render::*;
use errors::*;

struct MetalRenderPipeline {
    pipeline_state: metal::RenderPipelineState,
}

impl MetalRenderPipeline {}

pub struct MetalRenderBackend {
    device: metal::Device,
    command_queue: metal::CommandQueue,

    layer: Option<metal::CoreAnimationLayer>,
    dpi_factor: f32,

    frame_index: u64,
    time: Instant,
    time_last_frame: Instant,

    pipelines: Mutex<RefCell<Vec<MetalRenderPipeline>>>,
}

unsafe impl Sync for MetalRenderBackend {
    
}

impl MetalRenderBackend {
    pub fn new() -> MetalRenderBackend {
        let device = metal::Device::system_default();

        let command_queue = device.new_command_queue();

        MetalRenderBackend {
            device,
            command_queue,
            layer: None,
            dpi_factor: 1.0,
            frame_index: 0,
            time: Instant::now(),
            time_last_frame: Instant::now(),
            pipelines: Mutex::new(RefCell::new(vec![])),
        }
    }

    fn create_pipeline_state(&self, shader_source: &str) -> Result<metal::RenderPipelineState> {
        let compile_options = metal::CompileOptions::new();

        let vs_source = include_str!("shadertoy_vs.metal");
        let ps_source = shader_source;

        let vs_library = new_library_with_source(&self.device, vs_source, &compile_options)?;
        let ps_library = new_library_with_source(&self.device, ps_source, &compile_options)?;

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

        self.dpi_factor = winit_window.hidpi_factor();
    }

    fn render_frame(&mut self, params: RenderParams) {
        //println!("frame: {}", self.frame_index);

        if let Some(ref layer) = self.layer {
            if let Some(drawable) = layer.next_drawable() {
                let render_pass_descriptor = metal::RenderPassDescriptor::new();
                let color_attachment = render_pass_descriptor
                    .color_attachments()
                    .object_at(0)
                    .unwrap();
                color_attachment.set_texture(Some(drawable.texture()));
                color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                color_attachment.set_clear_color(metal::MTLClearColor::new(
                    params.clear_color.0 as f64,
                    params.clear_color.1 as f64,
                    params.clear_color.2 as f64,
                    params.clear_color.3 as f64,
                ));
                color_attachment.set_store_action(metal::MTLStoreAction::Store);

                let command_buffer = self.command_queue.new_command_buffer();
                let parallel_encoder = command_buffer.new_parallel_render_command_encoder(&render_pass_descriptor);
                let encoder = parallel_encoder.render_command_encoder();

                let w = drawable.texture().width() as f32;
                let h = drawable.texture().height() as f32;

                for quad in params.quads {
                    let constants = {
                        let time = self.time.elapsed().as_fractional_secs() as f32;
                        let delta_time = self.time_last_frame.elapsed().as_fractional_secs() as f32;

                        let dt: DateTime<Local> = Local::now();

                        let mut mouse = (
                            (params.mouse_pos.0 as f32) / self.dpi_factor,
                            (params.mouse_pos.1 as f32) / self.dpi_factor,
                            (params.mouse_click_pos.0 as f32) / self.dpi_factor,
                            (params.mouse_click_pos.1 as f32) / self.dpi_factor,
                        );

                        // flip y
                        if mouse.1 > 0.0 {
                            mouse.1 = h - mouse.1;
                        }
                        if mouse.3 > 0.0 {
                            mouse.3 = h - mouse.3;
                        }

                        ShadertoyConstants {
                            iResolution: ((quad.size.0 * w), (quad.size.1 * h), w / h),
                            pad1: 0.0,
                            iMouse: mouse,
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

                    let pipelines_lock = self.pipelines.lock().unwrap();
                    let pipelines = pipelines_lock.borrow();
                    let pipeline = &pipelines[quad.pipeline_handle];
                    let constants_ptr: *const ShadertoyConstants = &constants;
                    let constants_cptr = constants_ptr as *mut libc::c_void;

                    encoder.set_render_pipeline_state(&pipeline.pipeline_state);
                    encoder.set_cull_mode(metal::MTLCullMode::None);
                    encoder.set_vertex_bytes(0, mem::size_of::<ShadertoyConstants>() as u64, constants_cptr);
                    encoder.set_fragment_bytes(0, mem::size_of::<ShadertoyConstants>() as u64, constants_cptr);

                    encoder.set_viewport(metal::MTLViewport {
                        originX: (quad.pos.0 * w) as f64,
                        originY: (quad.pos.1 * h) as f64,
                        width: (quad.size.0 * w) as f64,
                        height: (quad.size.1 * h) as f64,
                        znear: 0.0,
                        zfar: 1.0,
                    });

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

    fn new_pipeline(&self, shader_source: &str) -> Result<RenderPipelineHandle> {
        let metal_source = convert_glsl_to_metal("unknown name", "main", shader_source)?;

        // save out the generated Metal file, for debugging
        // let msl_path = PathBuf::from(format!("output/shader/{} {}.metal", shadertoy, pass.name));
        // write_file(&msl_path, full_source_metal.as_bytes())?;

        let pipeline = MetalRenderPipeline {
            pipeline_state: self.create_pipeline_state(&metal_source)?,
        };

        let pipelines_lock = self.pipelines.lock().unwrap();
        let mut pipelines = pipelines_lock.borrow_mut();
        pipelines.push(pipeline);

        Ok(pipelines.len() - 1 as RenderPipelineHandle)
    }
}

// manually created version as the one in metal-rs will fail and return Err
// for shaders that just have compilation warnings
// TODO should figure out how to resolve this properly and merge it back?
fn new_library_with_source(device: &metal::Device, src: &str, options: &metal::CompileOptionsRef) -> Result<metal::Library> {
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
            return Ok(metal::Library::from_ptr(library));
        }

        if !err.is_null() {
            let desc: *mut Object = msg_send![err, localizedDescription];
            let compile_error: *const libc::c_char = msg_send![desc, UTF8String];
            let message = CStr::from_ptr(compile_error).to_string_lossy().into_owned();
            // original code crashes due to this release when having error message
            //msg_send![err, release];
            return Err(message.into());
        }

        return Err("unreachable?".into());
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
                // original code crashes due to this release when having error message
                //msg_send![$err_name, release];
                return Err(message.into());
            }
            value
        }
    };
}

// manually created version as the one in metal-rs will return Ok even if
// a null pipeline state is returnedfail and return Err
// TODO should merge this back
fn new_render_pipeline_state(
    device: &metal::Device,
    descriptor: &metal::RenderPipelineDescriptorRef,
) -> Result<metal::RenderPipelineState> {
    unsafe {
        let pipeline_state: *mut metal::MTLRenderPipelineState = try_objc!{ err =>
            msg_send![*device, newRenderPipelineStateWithDescriptor:descriptor
                                                            error:&mut err]
        };

        // This is the check that is new here
        // apparently there are cases where an error message is not returned but null is
        if pipeline_state.is_null() {
            return Err("newRenderPipelineStateWithDescriptor returned null".into());
        }

        Ok(metal::RenderPipelineState::from_ptr(pipeline_state))
    }
}

fn convert_glsl_to_metal(name: &str, entry_point: &str, source: &str) -> Result<String> {
    // convert to SPIR-V using shaderc

    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();

    let binary_result = compiler
        .compile_into_spirv(
            source,
            shaderc::ShaderKind::Fragment,
            name,
            entry_point,
            Some(&options),
        )
        .chain_err(|| "shaderc compilation to SPIRV failed")?;

    // convert SPIR-V to MSL

    use self::spirv_cross::{msl, spirv};

    let module = spirv::Module::from_words(binary_result.as_binary());

    let mut ast = spirv::Ast::<msl::Target>::parse(&module).unwrap();

    //    ast.compile().chain_err(|| "spirv-cross compilation failed")?

    match ast.compile() {
        Ok(str) => Ok(str),
        Err(e) => match e {
            spirv_cross::ErrorCode::Unhandled => Err("spirv-cross handled error".into()),
            spirv_cross::ErrorCode::CompilationError(str) => Err(format!("spirv-cross error: {}", str).into()),
        },
    }
}
