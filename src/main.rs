//! Small [Shadertoy](http://shadertoy.com) browser & viewer for Mac built in [Rust](https://www.rust-lang.org).
//! 
//! This application uses the [Shadertoy REST API](http://shadertoy.com/api) to search for Shadertoys and then downloads them locally and converts them using [`shaderc-rs`](https://crates.io/crates/shaderc) and [`spirv-cross`](https://crates.io/crates/spirv_cross) to be natively rendered on Mac using [`metal-rs`](https://crates.io/crates/metal-rs).
//! 
//! The API queries are done through the [`shadertoy`](https://crates.io/crates/shadertoy) crate, which can be found in  `src/shadertoy`

#![allow(dead_code)]
#![recursion_limit = "1024"] // `error_chain!` can recurse deeply

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate clap;

extern crate floating_duration;
extern crate chrono;
extern crate rayon;
extern crate winit;
extern crate open;
extern crate rust_base58 as base58;
extern crate serde_json;
extern crate colored;
extern crate indicatif;
#[macro_use]
extern crate thread_profiler;
extern crate reqwest;
extern crate shadertoy;

use std::io::Write;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant};
use floating_duration::{TimeAsFloat};
use clap::{Arg, App};   
use rayon::prelude::*;
use base58::ToBase58;
use indicatif::{ProgressBar, ProgressStyle};

mod render;
use render::*;


// TODO try and get rid of most of this and only depend on render_metal
#[cfg(target_os = "macos")]
mod render_metal;
#[cfg(target_os = "macos")]
use render_metal::*;
#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;
#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
use cocoa::foundation::NSAutoreleasePool;

mod errors {
    error_chain!{    
        links {
            Shadertoy(::shadertoy::Error, ::shadertoy::ErrorKind);
        }
    
        foreign_links {
            Fmt(::std::fmt::Error);
            Io(::std::io::Error);
            Json(::serde_json::error::Error);
            Clap(::clap::Error);
            Reqwest(::reqwest::Error);
        }
    }            
}
use errors::*;


struct BuiltShadertoy {

    info: shadertoy::ShaderInfo,

    //shader_path: String,
    //shader_source: String,
    pipeline_handle: RenderPipelineHandle,
}


fn write_file<P: AsRef<Path>>(path: P, buf: &[u8]) -> Result<()> {

    if let Some(parent_path) = path.as_ref().parent() {
        std::fs::create_dir_all(parent_path)?;
    } 
    
    let mut file = File::create(&path)?;
    file.write_all(buf)?;
    Ok(())
}

fn search(client: &shadertoy::Client, matches: &clap::ArgMatches) -> Result<Vec<String>> {

    profile_scope!("search");

    use std::str::FromStr;

    // create search parameters

    let search_params = shadertoy::SearchParams {
        string: matches.value_of("search").unwrap_or(""),
        
        sort_order: value_t!(matches, "order", shadertoy::SearchSortOrder)?,

        filters: match matches.values_of("filter") {
            Some(args) => args.map(|f| shadertoy::SearchFilter::from_str(f).unwrap()).collect(),
            None => vec![],
        },
   };

    println!("{:?}", search_params);

    // check if we can find a cached search on disk

    let path = format!("output/query/{}", serde_json::to_string(&search_params)?.as_bytes().to_base58());

    if let Ok(mut file) = File::open(&path) {
        let mut json_str = String::new();
        file.read_to_string(&mut json_str).chain_err(|| "failed reading json shader file")?;
        let search_result: serde_json::Result<Vec<String>> = serde_json::from_str(&json_str);
        search_result.chain_err(|| "shader query json deserialization failed")
    } else {
        // issue the actual request
        match client.search(&search_params).chain_err(|| "shadertoy search failed") {
            Ok(result) => {
                // cache search results to a file on disk
                write_file(&path, serde_json::to_string(&result)?.as_bytes())?;
                Ok(result)
            }
            Err(err) => Err(err),
        }
    }
}


fn download(
    matches: &clap::ArgMatches, 
    render_backend: &Option<Box<RenderBackend>>) -> Result<Vec<BuiltShadertoy>> 
{
    profile_scope!("download");
    
    let time = Instant::now();

    let api_key = matches.value_of("apikey").unwrap();
    let client = shadertoy::Client::new(api_key);
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("")); // workaround
    pb.enable_steady_tick(200);
    pb.tick(); // workaround for https://github.com/mitsuhiko/indicatif/issues/36
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green}  Searching{wide_msg}"));

    let shadertoys = search(&client, matches)?;
    let shadertoys_len = shadertoys.len();
        
    pb.finish_with_message(
        &format!(": {} found [{:.2} s]", shadertoys_len, time.elapsed().as_fractional_secs()));

    let built_shadertoys = Mutex::new(Vec::<BuiltShadertoy>::new());

    let pb = ProgressBar::new(shadertoys_len as u64);    
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} Processing [{bar:40.cyan/blue}] {pos}/{len} {eta}")
        .progress_chars("##-"));

    {
        // closure for processing a shadertoy    
        let process_shadertoy = |shadertoy| -> Result<()> {

            let path = PathBuf::from(format!("output/shader/{}/{}.json", shadertoy, shadertoy));

            let shader;

            if !path.exists() {
                profile_scope!("shader_json_query");
                shader = client.get_shader(shadertoy)?;
                write_file(&path, serde_json::to_string_pretty(&shader)?.as_bytes())?;
            } else {
                profile_scope!("shader_json_file_load");
                let mut json_str = String::new();
                File::open(&path)?.read_to_string(&mut json_str)?;
                shader = serde_json::from_str(&json_str)?;
            }
    /*
            println!("({} / {}) {} - {} by {} - {} views, {} likes", 
                index.fetch_add(1, Ordering::SeqCst), 
                shadertoys_len, 
                shadertoy.yellow(),
                shader.info.name.green(), 
                shader.info.username.blue(),
                shader.info.viewed,
                shader.info.likes);
    */
            //pb.set_message(&format!("\"{}\"", shader.info.name));

            for pass in &shader.renderpass {

                // generate a GLSL snippet containing the sampler declarations
                // as they are dependent on the renderpass inputs in the JSON
                // for exaxmple:
                //     uniform sampler2D iChannel0;
                //     uniform sampler2D iChannel1;
                //     uniform sampler2D iChannel2;
                //     uniform sampler2D iChannel3;

                let mut sampler_source = String::new();
                for input in &pass.inputs {
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
                let shader_path = format!("output/shader/{}/{}{}", shadertoy, shadertoy, pass.name);
                let glsl_path = format!("{}.glsl", shader_path);
                write_file(&glsl_path, full_source.as_bytes())?;

                // we currently only support single-pass image shaders, with no inputs
                if pass.pass_type == "image" && pass.inputs.is_empty() && shader.renderpass.len() == 1 {

                    // these shaders get stuck in forever compilation, so let's skip them forn ow
                    // TODO should make compilation more robust and be able to timeout and then remove this
                    let skip_shaders = [ "XdsBzj", "XtlSD7", "MlB3Wt", "4ssfzj", "XllSWf", "4td3z4" ];
                    
                    if skip_shaders.contains(&shader.info.id.as_str()) {
                        continue;
                    }

                    if let Some(ref rb) = *render_backend {

                        profile_scope!("new_pipeline");

                        match rb.new_pipeline(&shader_path, full_source.as_str()) {
                            Ok(pipeline_handle) => {
                                let mut bs = built_shadertoys.lock().unwrap();
                                bs.push(BuiltShadertoy {
                                    info: shader.info.clone(),
                                    pipeline_handle,
                                });
                            },
                            Err(_err) => {
                            /*
                                println!("Failed building shader for shadertoy {} ({} by {})", 
                                    shadertoy.info.id.yellow(),
                                    shadertoy.info.name.green(), 
                                    shadertoy.info.username.blue());
                            */
                            },
                        }
                    }
                }
                            
                // download texture inputs

                for input in &pass.inputs {

                    match input.ctype.as_str() {
                        "texture" | "volume" | "cubemap" | "buffer" => (),
                        _ => continue,
                    };

                    let path = PathBuf::from(format!("output{}", input.src));

                    if !path.exists() {

                        let mut data_response = client.rest_client
                            .get(&format!("https://www.shadertoy.com/{}", input.src))
                            .send()?;

                        let mut data = vec![];
                        data_response.read_to_end(&mut data)?;

                        //println!("Asset downloaded: {}, {} bytes", input.src, data.len());

                        write_file(&path, &data)?;
                    } 
                }
            }

            pb.inc(1);

            Ok(())
        };

        
        let threads: i64 = matches.value_of("threads").unwrap().parse().unwrap();

        if threads == 0 {
            for shadertoy in &shadertoys {
                let _r_ = process_shadertoy(shadertoy);
            }
        }
        else 
        {
            if threads > 1 {
                rayon::initialize(rayon::Configuration::new().num_threads(threads as usize)).unwrap();
            }

            let init_threads: Mutex<Vec<std::thread::ThreadId>> = Mutex::new(vec![]);

            shadertoys.par_iter().for_each(|shadertoy| {

                // TODO clean up & simplify this hacky way of naming the job worker threads
                {
                    let mut it = init_threads.lock().unwrap();
                    if !it.contains(&std::thread::current().id()) {                        
                        thread_profiler::register_thread_with_profiler(format!("job{}", it.len()));
                        it.push(std::thread::current().id());
                    }
                }

                let _r_ = process_shadertoy(shadertoy);
            });
        }
    }

    pb.finish_and_clear();

    let built_shadertoys = built_shadertoys.into_inner().unwrap();

    println!("  Processing: {} built successfully [{:.2} s]", 
        built_shadertoys.len(), 
        time.elapsed().as_fractional_secs());

    Ok(built_shadertoys)
}

fn run() -> Result<()> {

    let matches = App::new("Shadertoy Browser")
        .version(crate_version!())
        .author("Johan Andersson <repi@repi.se>")
        .about("Downloads and views shadertoys")
        .arg(
            Arg::with_name("apikey")
                .short("k")
                .long("apikey")
                .value_name("key")
                .default_value("BtHtWD") // be nice and have a default key so app just works
                .help("Set shadertoy API key to use. Create your key on https://www.shadertoy.com/myapps")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("search")
                .short("s")
                .long("search")
                .value_name("string")
                .help("Search string to filter which shadertoys to get")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("filter")
                .short("f")
                .long("filter")
                .help("Inclusion filters")
                .takes_value(true)
                .multiple(true)
                .possible_values(&["VR", "SoundOutput", "SoundInput", "Webcam", "MultiPass", "MusicStream"])
                .case_insensitive(true),
        )
        .arg(
            Arg::with_name("order")
                .short("o")
                .long("order")
                .help("Sort order")
                .takes_value(true)
                .default_value("Popular")
                .possible_values(&["Name", "Love", "Popular", "Newest", "Hot"])
                .case_insensitive(true),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .help("How many threads to use for downloading & processing shaders. 0 = disables threading, -1 = use all logical processors")
                .default_value("-1"),
        )
        .arg(
            Arg::with_name("headless")
                .short("h")
                .long("headless")
                .help("Don't render, only download shadertoys"),
        )
        .get_matches();

    if cfg!(target_os="windows") {
        // disable colored text output on Windows as the Windows terminals do not support it yet
        colored::control::set_override(false);
    }

    // setup renderer

    let render_backend: Option<Box<RenderBackend>>;

    #[cfg(target_os="macos")]
    {
        render_backend = Some(Box::new(MetalRenderBackend::new()));
    }

    #[cfg(not(target_os="macos"))]
    {
        render_backend = None;
    }

    thread_profiler::register_thread_with_profiler(String::from("main"));

    let mut built_shadertoy_shaders = download(&matches, &render_backend).chain_err(|| "query for shaders failed")?;

    thread_profiler::write_profile("profile-startup.json");

    if built_shadertoy_shaders.is_empty() {
        return Ok(());
    }

    let mut render_backend = render_backend.chain_err(|| "skipping rendering, as have no renderer available")?;

    if !matches.is_present("headless") {

        let mut events_loop = winit::EventsLoop::new();
        let window = winit::WindowBuilder::new()
            .with_dimensions(1024, 768)
            .with_title("Shadertoy Browser".to_string())
            .build(&events_loop)
            .chain_err(|| "error creating window")?;

        render_backend.init_window(&window);


        #[cfg(target_os="macos")]
        let mut pool = unsafe { NSAutoreleasePool::new(cocoa::base::nil) };

        let mut running = true;

        let mut mouse_pos = (0.0f64, 0.0f64);
        let mut mouse_pressed_pos = (0.0f64, 0.0f64);
        let mut mouse_click_pos = (0.0f64, 0.0f64);
        let mut mouse_lmb_pressed = false;
        

        let mut shadertoy_index = 0usize;
        let mut draw_grid = true;
        let grid_size = (4, 4);
        
        while running {

            let shadertoy_increment = if draw_grid {
                grid_size.0 * grid_size.1
            } else {
                1
            };

            events_loop.poll_events(|event| match event {
                winit::Event::WindowEvent { event: winit::WindowEvent::Closed, .. } => running = false,
                winit::Event::WindowEvent { event: winit::WindowEvent::CursorMoved { position, .. }, .. } => {
                    mouse_pos = position;
                    if mouse_lmb_pressed {
                        mouse_pressed_pos = position;
                    }
                }
                winit::Event::WindowEvent { event: winit::WindowEvent::MouseInput { state, button, .. }, .. } => {
                    if state == winit::ElementState::Pressed {
                        if button == winit::MouseButton::Left {
                            if !mouse_lmb_pressed {
                                mouse_click_pos = mouse_pos;
                            }
                            mouse_lmb_pressed = true;
                        }
                    }
                    else {
                        mouse_pressed_pos = (0.0, 0.0);
                        mouse_click_pos = (0.0, 0.0);
                        mouse_lmb_pressed = false;
                    }
                }
                winit::Event::WindowEvent { event: winit::WindowEvent::KeyboardInput { input, .. }, .. } => {
                    if input.state == winit::ElementState::Pressed {
                        match input.virtual_keycode {
                            Some(winit::VirtualKeyCode::Left) => {
                                shadertoy_index = shadertoy_index.saturating_sub(shadertoy_increment);
                            }
                            Some(winit::VirtualKeyCode::Right) => {
                                if shadertoy_index + shadertoy_increment < built_shadertoy_shaders.len() {
                                    shadertoy_index += shadertoy_increment;
                                }
                            }
                            Some(winit::VirtualKeyCode::Space) => {
                                draw_grid = !draw_grid;
                            }
                            Some(winit::VirtualKeyCode::Return) => {
                                if let Some(ref shadertoy) = built_shadertoy_shaders.get_mut(shadertoy_index) {
                                    let _r_ = open::that(format!("https://www.shadertoy.com/view/{}", shadertoy.info.id));
                                }
                            }
                            // this panics on Mac as "not yet implemented"
                        /*
                            Some(winit::VirtualKeyCode::F) => {
                                window.set_fullscreen(None);
                            }
                        */
                            // manual workaround for CMD-Q on Mac not quitting the app
                            // issue tracked in https://github.com/tomaka/winit/issues/41
                            Some(winit::VirtualKeyCode::Q) => {
                                if cfg!(target_os = "macos") && input.modifiers.logo {
                                    running = false;
                                }
                            }
                            _ => (),
                        }
                    }
                }
                winit::Event::WindowEvent { event: winit::WindowEvent::Resized { .. }, .. } => {
                    render_backend.init_window(&window);        
                }
                _ => (),
            });
            

            // render frame

            let mut quads: Vec<RenderQuad> = vec![];

            if draw_grid {

                let start_index = shadertoy_index / shadertoy_increment * shadertoy_increment;
                
                for index in 0..shadertoy_increment {

                    if let Some(shadertoy) = built_shadertoy_shaders.get(start_index + index) {

                        let grid_pos = (index % grid_size.0, index / grid_size.0 );

                        quads.push(RenderQuad {
                            pos: (
                                (grid_pos.0 as f32) / (grid_size.0 as f32), 
                                (grid_pos.1 as f32) / (grid_size.1 as f32)
                            ),
                            size: (
                                1.0 / (grid_size.0 as f32), 
                                1.0 / (grid_size.1 as f32)
                            ),
                            pipeline_handle: shadertoy.pipeline_handle,
                        });
                    }
                }
            } else if let Some(shadertoy) = built_shadertoy_shaders.get(shadertoy_index) {

                quads.push(RenderQuad {
                    pos: (0.0, 0.0),
                    size: (1.0, 1.0),
                    pipeline_handle: shadertoy.pipeline_handle,
                });
            }

            // update window title

            let active_shadertoy = built_shadertoy_shaders.get(shadertoy_index);

            if draw_grid && !built_shadertoy_shaders.is_empty() {
                window.set_title(&format!("Shadertoy ({} / {})", 
                    shadertoy_index+1, 
                    built_shadertoy_shaders.len()));
            } else if active_shadertoy.is_some() {
                window.set_title(&format!("Shadertoy ({} / {}) - {} by {}", 
                    shadertoy_index+1, 
                    built_shadertoy_shaders.len(), 
                    active_shadertoy.unwrap().info.name, 
                    active_shadertoy.unwrap().info.username));
            } else {
                window.set_title("Shadertoy Browser");
            }            

            render_backend.render_frame(RenderParams {
                clear_color: (0.0, 0.0, 0.0, 0.0),
                mouse_pos: mouse_pressed_pos,
                mouse_click_pos,
                quads: &quads
            });

            #[cfg(target_os = "macos")]
            unsafe {
                msg_send![pool, release];
                pool = NSAutoreleasePool::new(cocoa::base::nil);
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        use error_chain::ChainedError; // trait which holds `display_chain`
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
        ::std::process::exit(1);
    }
}
