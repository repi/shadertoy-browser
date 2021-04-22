#version 450

precision highp float;

layout(push_constant) uniform Glob {
    vec3 iResolution;
    vec4 iMouse;
    float iTime;
    float iTimeDelta;
    float iFrameRate;
    float iSampleRate;
    int iFrame;
    float iChannelTime[4];
    vec3 iChannelResolution[4];
    vec4 iDate;
    float iBlockOffset;
} glob;

layout(location = 0) out vec2 out_uv;

vec2 lerp(vec2 a, vec2 b, vec2 f) {
    return a + (b-a)*f;
}

void main() {
    vec2 uv = vec2(uvec2(gl_VertexIndex, gl_VertexIndex << 1) & 2);
    gl_Position = vec4(lerp(vec2(-1,-1), vec2(1, 1), uv), 0, 1);
    out_uv = uv * glob.iResolution.xy;
}

