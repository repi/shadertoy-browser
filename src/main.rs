extern crate reqwest;
extern crate json;
extern crate floating_duration;
extern crate rayon;
extern crate shaderc;
extern crate spirv_cross;
extern crate clap;

use clap::{Arg, App};
use std::io::Write;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path,PathBuf};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;

fn convert_glsl_to_metal(name: &str, entry_point: &str, source: &str) -> Result<String,String> {

    // convert to SPIR-V using shaderc

    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();

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

fn write_file(path: &Path, buf: &[u8]) {
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
                         .get_matches();


    let header_source = include_str!("shadertoy_header.glsl");
/*
    let shadertoy_source = include_str!("shadertoy_test.glsl");

    let source = format!("{}{}", header_source, shadertoy_source);

    let metal_shader = convert_glsl_to_metal("source.glsl", "main", source.as_str()).unwrap();

    println!("{}", metal_shader);
*/

    let api_key = matches.value_of("apikey").unwrap();

    let mut shadertoys: Vec<String> = vec![];
    {
        let client = reqwest::Client::new();

        let query_str: String = {
            if let Some(search_str) = matches.value_of("search") {
                format!("https://www.shadertoy.com/api/v1/shaders/query/{}?key={}", search_str, api_key)
            }
            else {
                format!("https://www.shadertoy.com/api/v1/shaders?key={}",api_key)
            }
        };

        let str = client.get(&query_str).send().unwrap().text().unwrap();

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

    let client = reqwest::Client::new();

    //for shadertoy in shadertoys {
    shadertoys.par_iter().for_each(|shadertoy| {
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

            let mut file = match File::create(&path) {
                Err(why) => panic!("couldn't create {:?}: {}", path, why.description()),
                Ok(file) => file,
            };

            let json = json::parse(&json_str).unwrap();
            json_str = json::stringify_pretty(json, 4);

            file.write_all(json_str.as_bytes()).unwrap();
        } 
        else {
            println!(
                "shadertoy ({} / {}): {}, already downloaded",
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


        let json = json::parse(&json_str).unwrap();

        for pass in json["Shader"]["renderpass"].members() {
            if let Some(code) = pass["code"].as_str() {

                let path = PathBuf::from(format!("output/{} {}.glsl", shadertoy, pass["name"].as_str().unwrap()));

                write_file(&path, code.as_bytes());
            }
        }

        index.fetch_add(1, Ordering::SeqCst);
    });
}
