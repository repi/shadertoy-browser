#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct glob
{
    float3 iResolution;
    float4 iMouse;
    float iTime;
    float iTimeDelta;
    float iFrameRate;
    float iSampleRate;
    int iFrame;
    float iChannelTime[4];
    float3 iChannelResolution[4];
    float4 iDate;
    float iBlockOffset;
};

struct VertexOutput
{
    float4 position [[position]];
    float2 uv [[user(locn0)]];
};

float2 lerp(float2 a, float2 b, float2 f)
{
    return a + (b-a)*f;
}

vertex VertexOutput vsMain(uint vertexId [[vertex_id]], constant glob& v_27 [[buffer(0)]])
{
    VertexOutput output;
    float2 uv = float2(uint2(vertexId, vertexId << 1) & 2);
    output.position = float4(lerp(float2(-1, -1), float2(1, 1), uv), 0, 1);
	output.uv = uv * v_27.iResolution.xy;
    return output;
}

