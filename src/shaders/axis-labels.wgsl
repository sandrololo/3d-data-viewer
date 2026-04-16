struct TransformationInput {
    col0: vec4<f32>,
    col1: vec4<f32>,
    col2: vec4<f32>,
    col3: vec4<f32>,
}
@group(0) @binding(0)
var<uniform> transformation: TransformationInput;

struct ProjectionInput {
    col0: vec4<f32>,
    col1: vec4<f32>,
    col2: vec4<f32>,
    col3: vec4<f32>,
}
@group(1) @binding(0)
var<uniform> projection: ProjectionInput;

struct ScreenSize {
    width: f32,
    height: f32,
}
@group(2) @binding(0)
var<uniform> screen_size: ScreenSize;

@group(3) @binding(0)
var font_texture: texture_2d<f32>;
@group(3) @binding(1)
var font_sampler: sampler;

struct LabelVertexInput {
    @location(0) anchor: vec3<f32>,
    @location(1) offset_px: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct LabelVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct LabelFragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) picking: vec2<u32>,
}

@vertex
fn vs_label(in: LabelVertexInput) -> LabelVertexOutput {
    let transformation_matrix = mat4x4<f32>(
        transformation.col0,
        transformation.col1,
        transformation.col2,
        transformation.col3
    );
    let projection_matrix = mat4x4<f32>(
        projection.col0,
        projection.col1,
        projection.col2,
        projection.col3
    );

    // Project anchor to clip space
    let clip = projection_matrix * transformation_matrix * vec4<f32>(in.anchor, 1.0);

    // Convert pixel offset to NDC offset (2 pixels / screen_size)
    let ndc_offset = vec2<f32>(
        in.offset_px.x * 2.0 / screen_size.width,
        in.offset_px.y * 2.0 / screen_size.height
    );

    // Apply screen-space offset in clip space (multiply by w to stay in clip coords)
    var out: LabelVertexOutput;
    out.position = vec4<f32>(
        clip.x + ndc_offset.x * clip.w,
        clip.y + ndc_offset.y * clip.w,
        clip.z,
        clip.w
    );
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_label(in: LabelVertexOutput) -> LabelFragmentOutput {
    let alpha = textureSample(font_texture, font_sampler, in.uv).r;
    if alpha < 0.5 {
        discard;
    }
    var out: LabelFragmentOutput;
    out.color = vec4<f32>(in.color.rgb, in.color.a * alpha);
    out.picking = vec2<u32>(0xFFFFFFFFu, 0xFFFFFFFFu);
    return out;
}
