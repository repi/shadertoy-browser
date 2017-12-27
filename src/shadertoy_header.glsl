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
