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

    //let shadertoy_source = include_str!("shadertoy_test.glsl");

    let header_source = String::from("
#version 440

layout(binding = 1, std140) uniform glob 
{
	uniform vec3	iResolution;
	uniform vec4	iMouse;
	uniform float	iTime;
	uniform float	iTimeDelta;
	uniform float	iFrameRate;
	uniform float	iSampleRate;
	uniform int	    iFrame;
	uniform float	iChannelTime[4];
	uniform vec3	iChannelResolution[4];
	uniform vec4	iDate;
};

uniform sampler2D iChannel0;
uniform sampler2D iChannel1;
uniform sampler2D iChannel2;
uniform sampler2D iChannel3;

void mainImage(out vec4 fragColor, in vec2 fragCoord);

layout(location = 0) in vec2 _fragCoord;
layout(location = 0) out vec4 _fragColor;

void main() 
{ 
	mainImage(_fragColor, _fragCoord); 
}

#define texture2D texture    
    ");

    let shadertoy_source = String::from("
//Seascape by Alexander Alekseev aka TDM - 2014
//License Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported License.
//Contact: tdmaav@gmail.com
//https://www.shadertoy.com/view/MdGGzy

const int NUM_STEPS = 8;
const float PI	 	= 3.1415;
const float EPSILON	= 1e-3;
float EPSILON_NRM	= 0.1 / iResolution.x;

// sea
const int ITER_GEOMETRY = 3;
const int ITER_FRAGMENT = 5;
const float SEA_HEIGHT = 0.6;
const float SEA_CHOPPY = 4.0;
const float SEA_SPEED = 0.8;
const float SEA_FREQ = 0.16;
const vec3 SEA_BASE = vec3(0.1,0.19,0.22);
const vec3 SEA_WATER_COLOR = vec3(0.8,0.9,0.6);
float SEA_TIME = iTime * SEA_SPEED;
mat2 octave_m = mat2(1.6,1.2,-1.2,1.6);

// math
mat3 fromEuler(vec3 ang) {
	vec2 a1 = vec2(sin(ang.x),cos(ang.x));
    vec2 a2 = vec2(sin(ang.y),cos(ang.y));
    vec2 a3 = vec2(sin(ang.z),cos(ang.z));
    mat3 m;
    m[0] = vec3(a1.y*a3.y+a1.x*a2.x*a3.x,a1.y*a2.x*a3.x+a3.y*a1.x,-a2.y*a3.x);
	m[1] = vec3(-a2.y*a1.x,a1.y*a2.y,a2.x);
	m[2] = vec3(a3.y*a1.x*a2.x+a1.y*a3.x,a1.x*a3.x-a1.y*a3.y*a2.x,a2.y*a3.y);
	return m;
}
float hash( vec2 p ) {
	float h = dot(p,vec2(127.1,311.7));	
    return fract(sin(h)*43758.5453123);
}
float noise( in vec3 uvt ) {
    vec2 p = uvt.xy;
    vec2 ft = fract(uvt.z * vec2(1.0, 1.0));
    vec2 i = floor(p+ft) + floor(uvt.z);
    vec2 f = fract( p +ft );
	vec2 u = f*f*(3.0-2.0*f);
    return -1.0+2.0*mix( mix( hash( i + vec2(0.0,0.0) ), 
                     hash( i + vec2(1.0,0.0) ), u.x),
                mix( hash( i + vec2(0.0,1.0) ), 
                     hash( i + vec2(1.0,1.0) ), u.x), u.y);
}

// lighting
float diffuse(vec3 n,vec3 l,float p) {
    return pow(dot(n,l) * 0.4 + 0.6,p);
}
float specular(vec3 n,vec3 l,vec3 e,float s) {    
    float nrm = (s + 8.0) / (3.1415 * 8.0);
    return pow(max(dot(reflect(e,n),l),0.0),s) * nrm;
}

// sky
vec3 getSkyColor(vec3 e) {
    e.y = max(e.y,0.0);
    vec3 ret;
    ret.x = pow(1.0-e.y,2.0);
    ret.y = 1.0-e.y;
    ret.z = 0.6+(1.0-e.y)*0.4;
    return ret;
}

// sea
float sea_octave(vec3 uvt, float choppy) {
    vec2 uv = uvt.xy;// + uvt.z;
    uv += noise(uvt);
    vec2 cost = cos(uvt.z * vec2(1.0, 1.0));
    vec2 sint = sin(uvt.z * vec2(1.0, 1.0));
    vec2 wv = 1.0-abs(sin(uv)*cost  + sint*cos(uv) );
    vec2 swv = abs(cos(uv)*cost - sin(uv)*sint);    
    
    wv = mix(wv,swv,wv);
    return pow(1.0-pow(wv.x * wv.y,0.65),choppy);
}

float map(vec3 p) {
    float freq = SEA_FREQ;
    float amp = SEA_HEIGHT;
    float choppy = SEA_CHOPPY;
    vec3 uvt = vec3(p.xz, SEA_TIME); uvt.x *= 0.75;
    
    float d, h = 0.0;    
    for(int i = 0; i < ITER_GEOMETRY; i++) {        
    	d = sea_octave((uvt)*freq,choppy);
    	d += sea_octave((uvt)*freq,choppy);
        h += d * amp;        
    	uvt.xy *= octave_m; freq *= 1.9; amp *= 0.22;
        choppy = mix(choppy,1.0,0.2);
    }
    return p.y - h;
}

float map_detailed(vec3 p) {
    float freq = SEA_FREQ;
    float amp = SEA_HEIGHT;
    float choppy = SEA_CHOPPY;
    vec3 uvt = vec3(p.xz, SEA_TIME); uvt.x *= 0.75;
    
    float d, h = 0.0;    
    for(int i = 0; i < ITER_FRAGMENT; i++) {        
    	d = sea_octave((uvt)*freq,choppy);
    	d += sea_octave((uvt)*freq,choppy);
        h += d * amp;        
    	uvt.xy*= octave_m; freq *= 1.9; amp *= 0.22;
        choppy = mix(choppy,1.0,0.2);
    }
    return p.y - h;
}

vec3 getSeaColor(vec3 p, vec3 n, vec3 l, vec3 eye, vec3 dist) {  
    float fresnel = 1.0 - max(dot(n,-eye),0.0);
    fresnel = pow(fresnel,3.0) * 0.65;
        
    vec3 reflected = getSkyColor(reflect(eye,n));    
    vec3 refracted = SEA_BASE + diffuse(n,l,80.0) * SEA_WATER_COLOR * 0.12; 
    
    vec3 color = mix(refracted,reflected,fresnel);
    
    float atten = max(1.0 - dot(dist,dist) * 0.001, 0.0);
    color += SEA_WATER_COLOR * (p.y - SEA_HEIGHT) * 0.18 * atten;
    
    color += vec3(specular(n,l,eye,60.0));
    
    return color;
}

// tracing
vec3 getNormal(vec3 p, float eps) {
    vec3 n;
    n.y = map_detailed(p);    
    n.x = map_detailed(vec3(p.x+eps,p.y,p.z)) - n.y;
    n.z = map_detailed(vec3(p.x,p.y,p.z+eps)) - n.y;
    n.y = eps;
    return normalize(n);
}

float heightMapTracing(vec3 ori, vec3 dir, out vec3 p) {  
    float tm = 0.0;
    float tx = 1000.0;    
    float hx = map(ori + dir * tx);
    if(hx > 0.0) return tx;   
    float hm = map(ori + dir * tm);    
    float tmid = 0.0;
    for(int i = 0; i < NUM_STEPS; i++) {
        tmid = mix(tm,tx, hm/(hm-hx));                   
        p = ori + dir * tmid;                   
    	float hmid = map(p);
		if(hmid < 0.0) {
        	tx = tmid;
            hx = hmid;
        } else {
            tm = tmid;
            hm = hmid;
        }
    }
    return tmid;
}

// main
void mainImage2( out vec4 fragColor, in vec2 fragCoord ) {
	vec2 uv = fragCoord.xy / iResolution.xy;
    uv = uv * 2.0 - 1.0;
    uv.x *= iResolution.x / iResolution.y;    
    float time = iTime * 0.3 + iMouse.x*0.01;
        
    // ray
    vec3 ang = vec3(sin(time*3.0)*0.1,sin(time)*0.2+0.3,time);    
    vec3 ori = vec3(0.0,3.5,time*5.0);
    vec3 dir = normalize(vec3(uv.xy,-2.0)); dir.z += length(uv) * 0.15;
    dir = normalize(dir) * fromEuler(ang);
    
    // tracing
    vec3 p;
    heightMapTracing(ori,dir,p);
    vec3 dist = p - ori;
    vec3 n = getNormal(p, dot(dist,dist) * EPSILON_NRM);
    vec3 light = normalize(vec3(0.0,1.0,0.8)); 
             
    // color
    vec3 color = mix(
        getSkyColor(dir),
        getSeaColor(p,n,light,dir,dist),
    	pow(smoothstep(0.0,-0.05,dir.y),0.3));
        
    // post
	fragColor = vec4(pow(color,vec3(0.75)), 1.0);
}


void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    float iTime = 0.0;
	vec2 uv = fragCoord.xy / iResolution.xy;
	fragColor = vec4(uv,0.5+0.5*sin(iTime),1.0);
}
");

    let source = format!("{}{}", header_source, shadertoy_source);

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();

    let binary_result = compiler.compile_into_spirv(
        source.as_str(),
        shaderc::ShaderKind::Fragment,
        "shader.glsl",
        "main",
        Some(&options)).unwrap();

    let text_result = compiler.compile_into_spirv_assembly(
        source.as_str(),
        shaderc::ShaderKind::Fragment,
        "shader.glsl",
        "main",
        Some(&options)).unwrap();


    {
        use spirv_cross::{spirv, hlsl, msl, ErrorCode};

        let module = spirv::Module::from_words(binary_result.as_binary());

        // Compile to MSL
        let mut ast = spirv::Ast::<msl::Target>::parse(&module).unwrap();
        println!("{}", ast.compile().unwrap());        
    }

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
