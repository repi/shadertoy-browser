extern crate reqwest;
extern crate json;
extern crate floating_duration;
extern crate rayon;
extern crate shaderc;
extern crate spirv_cross;
extern crate clap;


#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod shadertoy;

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
    let built_count = AtomicUsize::new(0);

    let client = reqwest::Client::new();


    //let mut built_shadertoys: Vec<String> = vec![];

    //for shadertoy in shadertoys.iter() {
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

            let entry_point = match pass.pass_type.as_str() {
                "sound" => "void main() { mainSound_(); }\n",
                _ => "void main() { mainImage_(); }\n",
            };            

            // add our header source first which includes shadertoy constant & resource definitions
            let full_source = format!("{}{}{}{}", header_source, sampler_source, entry_point, pass.code);

            // save out the source GLSL file, for debugging
            let glsl_path = PathBuf::from(format!("output/{} {}.glsl", shadertoy, pass.name));
            write_file(&glsl_path, full_source.as_bytes());

            match convert_glsl_to_metal(glsl_path.to_str().unwrap(), "main", full_source.as_str()) {
                Ok(full_source_metal) => {
                    // save out the generated Metal file, for debugging
                    let msl_path = PathBuf::from(format!("output/{} {}.metal", shadertoy, pass.name));
                    write_file(&msl_path, full_source_metal.as_bytes());                
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
            //built_shadertoys.push(shadertoy);
            built_count.fetch_add(1, Ordering::SeqCst);
        }

        index.fetch_add(1, Ordering::SeqCst);
    });

    println!("{} / {} shadertoys fully built", built_count.load(Ordering::SeqCst), shadertoys_len);
}
