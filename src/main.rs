#![allow(dead_code)]

extern crate reqwest;
extern crate json;
extern crate floating_duration;
extern crate chrono;
extern crate rayon;
extern crate shaderc;
extern crate spirv_cross;
extern crate clap;
extern crate winit;
extern crate rust_base58 as base58;

extern crate cocoa;
#[macro_use] extern crate objc;
extern crate objc_foundation;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate libc;
extern crate foreign_types;
extern crate metal_rs as metal;


mod shadertoy;

use clap::{Arg, App};
use std::io::Write;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path,PathBuf};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;
use cocoa::foundation::NSAutoreleasePool;
use std::ffi::CStr;
use objc::runtime::{Object, YES};
use foreign_types::ForeignType;

use cocoa::base::id as cocoa_id;
use cocoa::foundation::{NSSize};
use cocoa::appkit::{NSWindow, NSView};
use winit::os::macos::WindowExt;
use std::mem;
use std::any::Any;

use std::time::Instant;
use floating_duration::TimeAsFloat;
use chrono::prelude::*;
use base58::ToBase58;


#[allow(non_snake_case)]
struct ShadertoyConstants {
    // The viewport resolution (z is pixel aspect ratio, usually 1.0).
    iResolution: (f32, f32, f32),
    pad1: f32,
    /// xy contain the current pixel coords (if LMB is down). zw contain the click pixel.
    iMouse: (f32, f32, f32, f32),
    /// Current time in seconds.
    iTime: f32,
    /// Delta time since last frame.
    iTimeDelta: f32,
    /// Number of frames rendered per second.
    iFrameRate: f32,
    /// Sound sample rate (typically 44100).
    iSampleRate: f32, 
    /// Current frame
    iFrame: i32,
    pad2: [i32; 3],
    /// Year, month, day, time in seconds in .xyzw
    iDate: (f32, f32, f32, f32),      
    /// Time for channel (if video or sound), in seconds   
    iChannelTime: [f32; 4],
    /// Input texture resolution for each channel
    iChannelResolution: [(f32, f32, f32, f32); 4],
}

pub struct RenderParams {
    pub mouse_cursor_pos: (f64, f64),
    pub shader_source: String,
}

pub trait RenderBackend {
    fn init_window(&mut self, window: &Any);
    fn present(&mut self, params: RenderParams);
}

struct MetalRenderBackend {
    device: metal::Device,
    command_queue: metal::CommandQueue,

    layer: Option<metal::CoreAnimationLayer>,
    frame_index: u64,
    time: Instant,
    time_last_frame: Instant,

    shader_source: String,
    pipeline_state: Option<metal::RenderPipelineState>,
}

#[allow(non_snake_case)]
fn TEST_new_library_with_source(device: &metal::Device, src: &str, options: &metal::CompileOptionsRef) -> Result<metal::Library, String> {
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

        if !library.is_null() {
            return Result::Ok(metal::Library::from_ptr(library));
        }

        if !err.is_null() {
            let desc: *mut Object = msg_send![err, localizedDescription];
            let compile_error: *const ::libc::c_char = msg_send![desc, UTF8String];
            let message = CStr::from_ptr(compile_error).to_string_lossy().into_owned();
            msg_send![err, release];
            return Err(message);
        }

        return Err(String::from("unreachable?"));
    }
}


impl MetalRenderBackend {
    fn new() -> MetalRenderBackend {
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

    fn create_pipeline_state(&self, shader_source: String) -> Result<metal::RenderPipelineState,String> {
        let compile_options = metal::CompileOptions::new();
        
        let vs_source = include_str!("shadertoy_vs.metal");
        let ps_source = shader_source;

        let vs_library = TEST_new_library_with_source(&self.device, vs_source, &compile_options)?;
        let ps_library = TEST_new_library_with_source(&self.device, &ps_source, &compile_options)?;

        let vs = vs_library.get_function("vsMain", None)?;
        let ps = ps_library.get_function("main0", None)?;

        let vertex_desc = metal::VertexDescriptor::new();

        let pipeline_desc = metal::RenderPipelineDescriptor::new();
        pipeline_desc.set_vertex_function(Some(&vs));
        pipeline_desc.set_fragment_function(Some(&ps));
        pipeline_desc.set_vertex_descriptor(Some(vertex_desc));
        pipeline_desc.color_attachments().object_at(0).unwrap().set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);

        return self.device.new_render_pipeline_state(&pipeline_desc);
    }
   

    fn update_shader(&mut self, shader_source: String) {
   
        if shader_source == self.shader_source {
            return;
        }

        self.shader_source = shader_source.clone();

        self.pipeline_state = match self.create_pipeline_state(shader_source) {
            Ok(pipeline_state) => Some(pipeline_state),
            Err(string) => {
                println!("Error creating pipeline state: {}", string);
                None
            },
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

        if self.pipeline_state.is_none() {
            return;
        }

        //println!("frame: {}", self.frame_index);

        if let Some(ref layer) = self.layer {
            if let Some(drawable) = layer.next_drawable() {

                let mut constants = {
                    let w = drawable.texture().width() as f32;
                    let h = drawable.texture().height() as f32;

                    let time = self.time.elapsed().as_fractional_secs() as f32;
                    let delta_time = self.time_last_frame.elapsed().as_fractional_secs() as f32;

                    let dt: DateTime<Local> = Local::now();
                    
                    ShadertoyConstants {
                        iResolution: (w, h, w / h),
                        pad1: 0.0,
                        iMouse: (
                            params.mouse_cursor_pos.0 as f32,
                            params.mouse_cursor_pos.1 as f32,
                            0.0,
                            0.0),
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
                            dt.second() as f32 // TODO unclear what seconds should be here?
                        ), 
                        iChannelTime: [time, time, time, time], // TODO not correct
                        iChannelResolution: [
                            (0.0, 0.0, 0.0, 0.0),
                            (0.0, 0.0, 0.0, 0.0),
                            (0.0, 0.0, 0.0, 0.0),
                            (0.0, 0.0, 0.0, 0.0),
                        ],
                    }
                };
                
                let render_pass_descriptor = metal::RenderPassDescriptor::new();
                let color_attachment = render_pass_descriptor.color_attachments().object_at(0).unwrap();
                color_attachment.set_texture(Some(drawable.texture()));
                color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                color_attachment.set_clear_color(metal::MTLClearColor::new(((self.frame_index%100) as f64) / 100f64, 0.2, 0.2, 1.0));
                color_attachment.set_store_action(metal::MTLStoreAction::Store);
        
                let command_buffer = self.command_queue.new_command_buffer();
                let parallel_encoder = command_buffer.new_parallel_render_command_encoder(&render_pass_descriptor);
                let encoder = parallel_encoder.render_command_encoder();
                if let Some(ref pipeline_state) = self.pipeline_state {
                    encoder.set_render_pipeline_state(&pipeline_state);
                }
                encoder.set_cull_mode(metal::MTLCullMode::None);

                let constants_ptr: *const ShadertoyConstants = &constants;
                let constants_cptr = constants_ptr as *mut libc::c_void;
                encoder.set_vertex_bytes(0, std::mem::size_of::<ShadertoyConstants>() as u64, constants_cptr);
                encoder.set_fragment_bytes(0, std::mem::size_of::<ShadertoyConstants>() as u64, constants_cptr);

                encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 3);
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

fn convert_glsl_to_metal(name: &str, entry_point: &str, source: &str) -> Result<String,String> {

    // convert to SPIR-V using shaderc

    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();

    let binary_result = match compiler.compile_into_spirv(
        source,
        shaderc::ShaderKind::Fragment,
        name,
        entry_point,
        Some(&options)) {

        Ok(result) => result,
        Err(err) => {
            return Err(format!("shaderc compilation failed: {}", err));
        },
    };

/*
    let text_result = compiler.compile_into_spirv_assembly(
        source,
        shaderc::ShaderKind::Fragment,
        name,
        entry_point,
        Some(&options))?;
*/

    // convert SPIR-V to MSL

    use spirv_cross::{spirv, msl};

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

fn write_file(path: &Path, buf: &[u8]) {

    match path.parent() {
        Some(parent_path) => {
            match std::fs::create_dir_all(parent_path) {
                Err(why) => println!("couldn't create directory: {:?}", why.kind()),
                Ok(_) => {}
            }
        },
        _ => (),
    }
    
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {:?}: {}", path, why.description()),
        Ok(file) => file,
    };

    file.write_all(buf).unwrap();
}

fn main() {
    let matches = App::new("Shadertoy Downloader")
                         .version("0.2")
                         .author("Johan Andersson <repi@repi.se>")
                         .about("Downloads shadertoys as json files")
                         .arg(Arg::with_name("apikey")
                         	.short("k")
                         	.long("apikey")
                         	.value_name("key")
                            .required(true)
                         	.help("Set shadertoy API key to use. Create your key on https://www.shadertoy.com/myapps")
                            .takes_value(true))
                         .arg(Arg::with_name("search")
                         	.short("s")
                         	.long("search")
                         	.value_name("stringy")
                         	.help("Search string to filter which shadertoys to get")                         
                            .takes_value(true))
                        .arg(Arg::with_name("render")
                            .short("r")
                            .long("render")
                            .help("Render shadertoys in a window, otherwise will just download shadertoys"))
                         .get_matches();


    let api_key = matches.value_of("apikey").unwrap();

    let mut shadertoys: Vec<String> = vec![]; {

        let query_str: String = {
            if let Some(search_str) = matches.value_of("search") {
                format!("https://www.shadertoy.com/api/v1/shaders/query/{}?key={}", search_str, api_key)
            }
            else {
                format!("https://www.shadertoy.com/api/v1/shaders?key={}",api_key)
            }
        };

        let path = PathBuf::from(&format!("output/query/{}", query_str.as_bytes().to_base58()));

        let mut str;

        if path.exists() {
            let mut file = match File::open(&path) {
                Err(why) => panic!("couldn't open {:?}: {}", path, why.description()),
                Ok(file) => file,
            };

            str = String::new();
            file.read_to_string(&mut str).unwrap();                
        }
        else {
            let client = reqwest::Client::new();
            str = client.get(&query_str).send().unwrap().text().unwrap();
            write_file(&path, str.as_bytes());            
        }

        let json = json::parse(&str).unwrap();

        for v in json["Results"].members() {
            if let Some(shadertoy) = v.as_str() {
                shadertoys.push(String::from(shadertoy));
            }
        }
    }

    let shadertoys_len = shadertoys.len();

    println!("found {} shadertoys", shadertoys_len);

    match std::fs::create_dir_all("output") {
        Err(why) => println!("couldn't create directory: {:?}", why.kind()),
        Ok(_) => {}
    }

    let index = AtomicUsize::new(0);
    let built_count = AtomicUsize::new(0);

    let client = reqwest::Client::new();


    let mut built_shadertoy_shaders: Vec<String> = vec![];

    for shadertoy in shadertoys.iter() {
//    shadertoys.par_iter().for_each(|shadertoy| {

        index.fetch_add(1, Ordering::SeqCst);

        let path = PathBuf::from(format!("output/{}.json", shadertoy));

        let mut json_str: String;

        if !path.exists() {
            json_str = client.get(&format!("https://www.shadertoy.com/api/v1/shaders/{}?key={}", shadertoy, api_key)).send().unwrap().text().unwrap();

            println!(
                "shadertoy ({} / {}): {}, json size: {}",
                index.load(Ordering::SeqCst),
                shadertoys_len,
                shadertoy,
                json_str.len()
            );

            let json: shadertoy::Root = serde_json::from_str(&json_str).unwrap();
            json_str = serde_json::to_string_pretty(&json).unwrap();            

            write_file(&path, json_str.as_bytes());
        } 
        else {
            println!(
                "shadertoy ({} / {}): {}",
                index.load(Ordering::SeqCst),
                shadertoys_len,
                shadertoy
            );

            let mut file = match File::open(&path) {
                Err(why) => panic!("couldn't open {:?}: {}", path, why.description()),
                Ok(file) => file,
            };

            json_str = String::new();
            file.read_to_string(&mut json_str).unwrap();
        }


        let root: shadertoy::Root = serde_json::from_str(&json_str).unwrap();

        let mut success = true;

        for pass in root.shader.renderpass.iter() {

            // generate a GLSL snippet containing the sampler declarations
            // as they are dependent on the renderpass inputs in the JSON
            // for exaxmple:
            //     uniform sampler2D iChannel0;
            //     uniform sampler2D iChannel1;
            //     uniform sampler2D iChannel2;
            //     uniform sampler2D iChannel3;

            let mut sampler_source = String::new();
            for input in pass.inputs.iter() {
                let glsl_type = match input.ctype.as_str() {
                    "texture" => "sampler2D",
                    "volume" => "sampler3D",
                    "cubemap" => "samplerCube",
                    "buffer" => "sampler2D",
                    "video" => "sampler2D",
                    "webcam" => "sampler2D",
                    "keyboard" => "sampler2D",
                    "music" => "sampler2D",
                    "musicstream" => "sampler2D",
                    "mic" => "sampler2D",
                    _ => {
                        panic!("Unknown ctype: {}", input.ctype); 
                    }
                };
                sampler_source.push_str(&format!("uniform {} iChannel{};\n", glsl_type, input.channel));
            }

            let header_source = include_str!("shadertoy_header.glsl");
            let image_footer_source = include_str!("shadertoy_image_footer.glsl");
            let sound_footer_source = include_str!("shadertoy_sound_footer.glsl");

            let footer_source = match pass.pass_type.as_str() {
                "sound" => sound_footer_source,
                _ => image_footer_source,
            };            

            // add our header source first which includes shadertoy constant & resource definitions
            let full_source = format!("{}\n{}\n{}\n{}", header_source, sampler_source, pass.code, footer_source);

            // save out the source GLSL file, for debugging
            let glsl_path = PathBuf::from(format!("output/{} {}.glsl", shadertoy, pass.name));
            write_file(&glsl_path, full_source.as_bytes());

            match convert_glsl_to_metal(glsl_path.to_str().unwrap(), "main", full_source.as_str()) {
                Ok(full_source_metal) => {
                    // save out the generated Metal file, for debugging
                    let msl_path = PathBuf::from(format!("output/{} {}.metal", shadertoy, pass.name));
                    write_file(&msl_path, full_source_metal.as_bytes());                

                    if pass.pass_type == "image" && pass.inputs.len() == 0 {
                        built_shadertoy_shaders.push(full_source_metal);
                    }
                }
                Err(string) => {
                    success = false;
                    println!("Failed compiling shader {}: {}", glsl_path.to_str().unwrap(), string);
                }
            }

            // download texture inputs

            for input in pass.inputs.iter() {

                match input.ctype.as_str() {
                    "texture" | "volume" | "cubemap" | "buffer" => (),
                    _ => continue,
                };

                let path = PathBuf::from(format!("output{}", input.src));

                if !path.exists() {

                    let mut data_response = client.get(&format!("https://www.shadertoy.com/{}", input.src)).send().unwrap();
                    
                    let mut data = vec![];
                    data_response.read_to_end(&mut data).unwrap();

                    println!("Asset downloaded: {}, {} bytes", input.src, data.len());

                    write_file(&path, &data);
                }
                else {

                    if let Ok(metadata) = path.metadata() {
                        println!("Asset: {}, {} bytes", input.src, metadata.len());
                    }
                }

            }
        }

        if success {
            built_count.fetch_add(1, Ordering::SeqCst);
        }
//    });
    }

    println!("{} / {} shadertoys fully built", built_count.load(Ordering::SeqCst), shadertoys_len);



    if built_shadertoy_shaders.len() == 0 {
        return;
    }

    if matches.is_present("render") {

        let mut events_loop = winit::EventsLoop::new();
        let window = winit::WindowBuilder::new()
            .with_dimensions(1024, 768)
            .with_title("Metal".to_string())
            .build(&events_loop).unwrap();

        let mut render_backend = MetalRenderBackend::new();
        render_backend.init_window(&window);



        let mut pool = unsafe { NSAutoreleasePool::new(cocoa::base::nil) };

        let mut running = true;
  
        let mut cursor_pos = (0.0f64, 0.0f64);
        let mut shadertoy_index = 0usize;

        while running {

            events_loop.poll_events(|event| {
                match event {
                    winit::Event::WindowEvent{ event: winit::WindowEvent::Closed, .. } => running = false,
                    winit::Event::WindowEvent{ event: winit::WindowEvent::CursorMoved { position, .. }, .. } => {
                        cursor_pos = position;
                    },
                    winit::Event::WindowEvent{ event: winit::WindowEvent::KeyboardInput { input, .. }, .. } => {
                        if input.state == winit::ElementState::Pressed {
                            match input.virtual_keycode {
                                Some(winit::VirtualKeyCode::Left) => {
                                    if shadertoy_index != 0 {
                                        shadertoy_index -= 1;
                                    }
                                },
                                Some(winit::VirtualKeyCode::Right) => {
                                    if shadertoy_index+1 < built_shadertoy_shaders.len() {
                                        shadertoy_index += 1;
                                    }
                                },
                                _ => (),
                            }
                        }
                    },
                    _ => (),
                }
            });


            render_backend.present(RenderParams {
                mouse_cursor_pos: cursor_pos,
                shader_source: built_shadertoy_shaders[shadertoy_index].clone()
            });

            unsafe { 
                msg_send![pool, release];
                pool = NSAutoreleasePool::new(cocoa::base::nil);
            }            
        }
    }    
}
