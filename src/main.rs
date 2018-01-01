//! Small [Shadertoy](http://shadertoy.com) browser & viewer for Mac built in [Rust](https://www.rust-lang.org).
//! 
//! This application uses the [Shadertoy REST API](http://shadertoy.com/api) to search for Shadertoys and then downloads them locally and converts them using [`shaderc-rs`](https://crates.io/crates/shaderc) and [`spirv-cross`](https://crates.io/crates/spirv_cross) to be natively rendered on Mac using [`metal-rs`](https://crates.io/crates/metal-rs).
//! 
//! The API queries are done through the [`shadertoy`](https://crates.io/crates/shadertoy) crate, which can be found in  `src/shadertoy`

#![allow(dead_code)]

extern crate shadertoy;

#[macro_use]
extern crate clap;

extern crate floating_duration;
extern crate chrono;
extern crate rayon;
extern crate winit;
extern crate rust_base58 as base58;
extern crate serde_json;
extern crate colored;

use colored::*;
use clap::{Arg, App};
use std::io::Write;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
//use std::thread;
//use rayon::prelude::*;

//use chrono::prelude::*;
use base58::ToBase58;

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

fn write_file(path: &Path, buf: &[u8]) {

    match path.parent() {
        Some(parent_path) => {
            match std::fs::create_dir_all(parent_path) {
                Err(why) => println!("couldn't create directory: {:?}", why.kind()),
                Ok(_) => {}
            }
        }
        _ => (),
    }

    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {:?}: {}", path, why.description()),
        Ok(file) => file,
    };

    let _ = file.write_all(buf);
}


fn search(client: &shadertoy::Client, matches: &clap::ArgMatches) -> Result<Vec<String>, Box<std::error::Error>> {

    //use shadertoy::SearchFilter::FromStr;
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

    let path = PathBuf::from(&format!("output/query/{}", serde_json::to_string(&search_params)?.as_bytes().to_base58()));

    if let Ok(mut file) = File::open(&path) {
        let mut json_str = String::new();
        file.read_to_string(&mut json_str)?;
        let search_result: serde_json::Result<Vec<String>> = serde_json::from_str(&json_str);
        match search_result {
            Ok(r) => Ok(r),
            Err(err) => Err(Box::new(err)),
        }
    } else {
        // issue the actual request
        match client.search(search_params) {
            Ok(result) => {
                // cache search results to a file on disk
                write_file(&path, serde_json::to_string(&result)?.as_bytes());
                Ok(result)
            }
            Err(err) => Err(err),
        }
    }
}

fn query(matches: &clap::ArgMatches, sender: std::sync::mpsc::Sender<String>) -> Result<(), Box<std::error::Error>> {

    let api_key = matches.value_of("apikey").unwrap();
    let client = shadertoy::Client::new(api_key);

    let shadertoys = search(&client, matches)?;

    let shadertoys_len = shadertoys.len();

    println!("found {} shadertoys", shadertoys_len);

    std::fs::create_dir_all("output")?;

    let index = AtomicUsize::new(0);
    let built_count = AtomicUsize::new(0);


    for shadertoy in shadertoys.iter() {
        //    shadertoys.par_iter().for_each(|shadertoy| {

        let path = PathBuf::from(format!("output/shader/{}.json", shadertoy));

        let mut shader;

        if !path.exists() {
            shader = client.get_shader(shadertoy)?;
            write_file(&path, serde_json::to_string_pretty(&shader)?.as_bytes());
        } else {
            let mut json_str = String::new();
            File::open(&path)?.read_to_string(&mut json_str)?;
            shader = serde_json::from_str(&json_str)?;
        }

        println!("({} / {}) {} - {} by {}, {} views, {} likes", 
            index.fetch_add(1, Ordering::SeqCst), 
            shadertoys_len, 
            shadertoy,
            shader.info.name.green(), 
            shader.info.username.blue(),
            shader.info.viewed,
            shader.info.likes);

        let mut success = true;

        for pass in shader.renderpass.iter() {

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
            let glsl_path = PathBuf::from(format!("output/shader/{} {}.glsl", shadertoy, pass.name));
            write_file(&glsl_path, full_source.as_bytes());

            #[cfg(target_os = "macos")]
            match convert_glsl_to_metal(glsl_path.to_str().unwrap(), "main", full_source.as_str()) {
                Ok(full_source_metal) => {
                    // save out the generated Metal file, for debugging
                    let msl_path = PathBuf::from(format!("output/shader/{} {}.metal", shadertoy, pass.name));
                    write_file(&msl_path, full_source_metal.as_bytes());

                    if pass.pass_type == "image" && pass.inputs.len() == 0 {
                        // sent built shader
                        sender.send(full_source_metal)?;
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

                    let mut data_response = client.rest_client
                        .get(&format!("https://www.shadertoy.com/{}", input.src))
                        .send()?;

                    let mut data = vec![];
                    data_response.read_to_end(&mut data)?;

                    println!("Asset downloaded: {}, {} bytes", input.src, data.len());

                    write_file(&path, &data);
                } else {

                /*
                    if let Ok(metadata) = path.metadata() {
                        println!("Asset: {}, {} bytes", input.src, metadata.len());
                    }
                */
                }

            }
        }

        if success {
            built_count.fetch_add(1, Ordering::SeqCst);
        }
        //    });
    }

    println!("{} / {} shadertoys fully built", built_count.load(Ordering::SeqCst), shadertoys_len);

    Ok(())
}

fn run(matches: &clap::ArgMatches) {
    let (sender, receiver) = std::sync::mpsc::channel::<String>();

    if let Err(err) = query(matches, sender.clone()) {
        println!("Query failed: {}", err);
        std::process::exit(1);
    }

    if !matches.is_present("headless") {

        let mut events_loop = winit::EventsLoop::new();
        let window = winit::WindowBuilder::new()
            .with_dimensions(1024, 768)
            .with_title("Shadertoy Browser".to_string())
            .build(&events_loop)
            .unwrap();

        let mut render_backend: Option<Box<RenderBackend>> = None;

        #[cfg(target_os = "macos")]
        {
            render_backend = Some(Box::new(MetalRenderBackend::new()));
        }

        let mut render_backend = render_backend.expect("No renderer available");
        render_backend.init_window(&window);


        #[cfg(target_os="macos")]
        let mut pool = unsafe { NSAutoreleasePool::new(cocoa::base::nil) };

        let mut running = true;

        let mut cursor_pos = (0.0f64, 0.0f64);

        let mut shadertoy_index = 0usize;
        let mut built_shadertoy_shaders: Vec<String> = Vec::new();

        while running {

            while let Ok(shader_source) = receiver.try_recv() {
                built_shadertoy_shaders.push(shader_source);
            }

            events_loop.poll_events(|event| match event {
                winit::Event::WindowEvent { event: winit::WindowEvent::Closed, .. } => running = false,
                winit::Event::WindowEvent { event: winit::WindowEvent::CursorMoved { position, .. }, .. } => {
                    cursor_pos = position;
                }
                winit::Event::WindowEvent { event: winit::WindowEvent::KeyboardInput { input, .. }, .. } => {
                    if input.state == winit::ElementState::Pressed {
                        match input.virtual_keycode {
                            Some(winit::VirtualKeyCode::Left) => {
                                if shadertoy_index != 0 {
                                    shadertoy_index -= 1;
                                }
                            }
                            Some(winit::VirtualKeyCode::Right) => {
                                if shadertoy_index + 1 < built_shadertoy_shaders.len() {
                                    shadertoy_index += 1;
                                }
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            });

            render_backend.present(RenderParams {
                mouse_cursor_pos: cursor_pos,
                shader_source: {
                    if let Some(shader_source) = built_shadertoy_shaders.get(shadertoy_index) {
                        shader_source.clone()
                    } else {
                        String::new() // empty string
                    }
                },
            });

            #[cfg(target_os = "macos")]
            unsafe {
                msg_send![pool, release];
                pool = NSAutoreleasePool::new(cocoa::base::nil);
            }
        }
    }
}

fn main() {
    let matches = App::new("Shadertoy Browser")
        .version(crate_version!())
        .author("Johan Andersson <repi@repi.se>")
        .about("Downloads shadertoys as json files") // TODO update about
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
            Arg::with_name("headless")
                .short("h")
                .long("headless")
                .help("Don't render, only download shadertoys"),
        )
        .get_matches();

    run(&matches);
}
