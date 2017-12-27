extern crate reqwest;
extern crate json;
extern crate floating_duration;
extern crate rayon;
extern crate shaderc;
extern crate spirv_cross;
extern crate clap;

use clap::{Arg, App};
use std::io::Write;
use std::fs::File;
use std::path::PathBuf;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;

fn convert_glsl_to_metal(name: &str, entry_point: &str, source: &str) -> Result<String,String> {

    // convert to SPIR-V using shaderc

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();

    let binary_result = compiler.compile_into_spirv(
        source,
        shaderc::ShaderKind::Fragment,
        name,
        entry_point,
        Some(&options)).unwrap();
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
                spirv_cross::ErrorCode::Unhandled => Err(String::from("Unhandled spirv-cross compiler error")),
                spirv_cross::ErrorCode::CompilationError(str) => Err(str),
            }
        }
    }
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
                         	.help("Set shadertoy API key to use. Create your key on https://www.shadertoy.com/myapps")
                         .takes_value(true))
                         .get_matches();


    let header_source = include_str!("shadertoy_header.glsl");
    let shadertoy_source = include_str!("shadertoy_test.glsl");

    let source = format!("{}{}", header_source, shadertoy_source);

    let metal_shader = convert_glsl_to_metal("source.glsl", "main", source.as_str()).unwrap();

    println!("{}", metal_shader);

    return;


    let api_key = matches.value_of("apikey").unwrap();

    let mut shadertoys: Vec<String> = vec![];
    {
        let client = reqwest::Client::new();
        let str = client.get(&format!("https://www.shadertoy.com/api/v1/shaders?key={}",api_key)).send().unwrap().text().unwrap();

        let shaders_json = json::parse(&str).unwrap();

        for v in shaders_json["Results"].members() {
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

    let mut index = AtomicUsize::new(0);

    let client = reqwest::Client::new();

    //for shadertoy in shadertoys {
    shadertoys.par_iter().for_each(|shadertoy| {
        let path = PathBuf::from(format!("output/{}.json", shadertoy));
        if !path.exists() {
            let str = client.get(&format!("https://www.shadertoy.com/api/v1/shaders/{}?key={}", shadertoy, api_key)).send().unwrap().text().unwrap();

            println!(
                "shadertoy ({} / {}): {}, json size: {}",
                index.load(Ordering::SeqCst),
                shadertoys_len,
                shadertoy,
                str.len()
            );

            // Open a file in write-only mode, returns `io::Result<File>`
            let mut file = match File::create(&path) {
                Err(why) => panic!("couldn't create {:?}: {}", path, why.description()),
                Ok(file) => file,
            };

            file.write_all(str.as_bytes());
        } else {
            println!(
                "shadertoy ({} / {}): {}, already downloaded - skipping",
                index.load(Ordering::SeqCst),
                shadertoys_len,
                shadertoy
            );
        }
        index.fetch_add(1, Ordering::SeqCst);
    });
}
