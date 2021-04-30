#version 440

precision highp float;

layout(push_constant) uniform glob
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
	uniform float   iBlockOffset;
};

#define texture2D texture
