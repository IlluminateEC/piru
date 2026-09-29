// -*- mode: hlsl -*-

struct VSInput {

};

struct VSOutput {
  float4 position : SV_POSITION;
  float2 uv : TEXCOORD0;
};

struct RectMetadata {
  float width;
  float height;
};

// [[vk::binding(0, 0)]] StructuredBuffer<RectMetadata> rect_metadata;

static const RectMetadata rect_metadata[2] = {
    { 0.25, 0.25 },
    { -0.1, -0.1 },
};

struct RectPosition {
  float x;
  float y;
  int index;
};

static const RectPosition rect_positions[2] = {
    { 0.25, 0.25, 0 },
    { 0.25, 0.25, 1 },
};

// [[vk::binding(1, 0)]] StructuredBuffer<RectPosition> rect_positions;

const static int VERTICES_IN_A_RECTANGLE = 6;

VSOutput main(VSInput input, uint vertex_index : SV_VertexID) {
  VSOutput output = (VSOutput) 0;

  RectPosition position = rect_positions[vertex_index / VERTICES_IN_A_RECTANGLE];
  RectMetadata metadata = rect_metadata[position.index];

  float2 top_left = float2(position.x, position.y);
  float2 top_right = float2(position.x + metadata.width, position.y);
  float2 bottom_left = float2(position.x, position.y - metadata.height);
  float2 bottom_right = float2(position.x + metadata.width, position.y - metadata.height);

  int vertex_number_within_rect = vertex_index % VERTICES_IN_A_RECTANGLE;

  float2 options[6] = {
    top_left, top_right, bottom_left,

    bottom_left, top_right, bottom_right,
  };

  float2 option = options[vertex_number_within_rect];
  output.position = float4(option.x, option.y, 0.0, 1.0);

  float2 top_left_uv = float2(0.0, 0.0);
  float2 bottom_left_uv = float2(0.0, 1.0);
  float2 top_right_uv = float2(1.0, 0.0);
  float2 bottom_right_uv = float2(1.0, 1.0);

  float2 uvs[6] = {
    top_left_uv, top_right_uv, bottom_left_uv,

    bottom_left_uv, top_right_uv, bottom_right_uv,
  };

  output.uv = uvs[vertex_number_within_rect];

  return output;
}
