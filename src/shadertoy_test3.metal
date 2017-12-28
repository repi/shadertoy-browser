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

constant float _92 = float(0);

struct main0_in
{
    float2 _fragCoord [[user(locn0)]];
};

struct main0_out
{
    float4 _fragColor [[color(0)]];
};

float Rectangle(thread const float2& p, thread const float2& min_, thread const float2& max_)
{
    float k = min(p.x - min_.x, p.y - min_.y);
    k = min(k, max_.x - p.x);
    float _112 = k;
    float _118 = min(_112, max_.y - p.y);
    k = _118;
    return _118;
}

float Quadrant(thread const float2& p, thread const float& r, thread const int& x, thread const int& y, thread const bool& inside, thread const float& outerRadiusProtection)
{
    float q = step(max((-float(x)) * p.x, (-float(y)) * p.y), 0.0);
    if (inside)
    {
        return (1.0 - (((1.0 - r) + length(p)) * q)) * step(length(p) + (outerRadiusProtection * r), 1.0);
    }
    else
    {
        return ((length(p) - r) * q) * step(length(p) + (outerRadiusProtection * r), 1.0);
    }
}

void mainImage(thread float4& col, thread const float2& coord, constant glob& v_130)
{
    float border = 0.920000016689300537109375;
    float e = 1.0 / min(v_130.iResolution.x, v_130.iResolution.y);
    float intro = smoothstep(0.0, 1.0, v_130.iTime / 2.0);
    float prec = mix(32.0, 3.0, intro) * e;
    float2 p = ((coord - (v_130.iResolution.xy / float2(2.0))) * e) * mix(24.0, 2.099999904632568359375, intro);
    float2 param = p;
    float2 param_1 = float2(-1.0);
    float2 param_2 = float2(1.0);
    float k = Rectangle(param, param_1, param_2);
    float2 param_3 = abs(p) - float2(border);
    float param_4 = 1.0 - border;
    int param_5 = 1;
    int param_6 = 1;
    bool param_7 = true;
    float param_8 = -10.0;
    k = min(k, Quadrant(param_3, param_4, param_5, param_6, param_7, param_8));
    float2 param_9 = p;
    float2 param_10 = float2(0.064000003039836883544921875, -1.0);
    float2 param_11 = float2(0.384000003337860107421875, 0.688000023365020751953125);
    k = min(k, -Rectangle(param_9, param_10, param_11));
    float2 param_12 = p;
    float2 param_13 = float2(0.0719999969005584716796875, 0.41600000858306884765625);
    float2 param_14 = float2(0.688000023365020751953125);
    k = min(k, -Rectangle(param_12, param_13, param_14));
    float2 pm = p;
    if (pm.x > 0.0)
    {
        pm.x -= (pm.y * 0.119999997317790985107421875);
    }
    float2 param_15 = pm;
    float2 param_16 = float2(-0.1920000016689300537109375, -0.21600000560283660888671875);
    float2 param_17 = float2(0.680000007152557373046875, 0.0719999969005584716796875);
    k = min(k, -Rectangle(param_15, param_16, param_17));
    float2 param_18 = p - float2(0.3919999897480010986328125, 0.36000001430511474609375);
    float param_19 = 0.328000009059906005859375;
    int param_20 = -1;
    int param_21 = 1;
    bool param_22 = false;
    float param_23 = 1.5;
    k = max(k, Quadrant(param_18, param_19, param_20, param_21, param_22, param_23));
    col = mix(float4(1.0), float4(0.23000000417232513427734375, 0.3499999940395355224609375, 0.60000002384185791015625, 1.0), float4(smoothstep(0.0, prec, k)));
    float2 param_24 = p - float2(0.4975999891757965087890625, 0.30239999294281005859375);
    float param_25 = 0.10999999940395355224609375;
    int param_26 = -1;
    int param_27 = 1;
    bool param_28 = false;
    float param_29 = -2.5;
    col = mix(col, float4(1.0), float4(smoothstep(0.0, prec, (Quadrant(param_24, param_25, param_26, param_27, param_28, param_29) * step(-p.x, -0.300000011920928955078125)) * step(p.y, 0.5))));
    col = mix(float4(1.0), col, float4(intro));
}

fragment main0_out main0(main0_in in [[stage_in]], constant glob& v_130 [[buffer(0)]])
{
    main0_out out = {};
    float2 param_1 = in._fragCoord;
    float4 param;
    mainImage(param, param_1, v_130);
    out._fragColor = param;
    return out;
}

