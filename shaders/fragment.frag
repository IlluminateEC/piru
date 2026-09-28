// -*- mode: hlsl -*-

#include "smooth_utils.hlsl"

struct FSInput {
  float2 uv : TEXCOORD0;
};

struct FSOutput {
  [[vk::location(0)]] float4 color : SV_TARGET0;
};

static const float4 PRIMARY = float4(255.0 / 255, 255.0 / 255, 255.0 / 255, 1);
static const float4 SECONDARY = float4(0.0 / 255, 0.0 / 255, 0.0 / 255, 1);

FSOutput main(FSInput input) {
  FSOutput output = (FSOutput) 0;

  float factor = smooth_modulo((input.uv.x + input.uv.y) / 2.0 * 5.0, 1.0);
  output.color = lerp(PRIMARY, SECONDARY, smoothed_condition(factor, 0.5));

  output.color.a = smoothed_circle(input.uv, float2(0.5, 0.5), 0.5);

  return output;
}
