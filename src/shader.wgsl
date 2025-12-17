struct VertexInput {
    @location(0) index: u32,
}
@group(0) @binding(0)
var surface_texture: texture_2d<f32>;
@group(0) @binding(1)
var amplitude_texture: texture_2d<f32>;
@group(0) @binding(2)
var overlay_texture: texture_2d<f32>;

struct ImageDimensions {
    width: u32,
    height: u32,
}
@group(1) @binding(0)
var<uniform> image_dims: ImageDimensions;

struct ZValueRange{
     min: f32,
     max: f32,
}
@group(1) @binding(1)
var<uniform> z_range: ZValueRange;

@group(1) @binding(2)
var<uniform> mip_level: u32;

struct TransformationInput {
    col0: vec4<f32>,
    col1: vec4<f32>,
    col2: vec4<f32>,
    col3: vec4<f32>,
}
@group(2) @binding(0)
var<uniform> transformation: TransformationInput;

struct ProjectionInput{
    col0: vec4<f32>,
    col1: vec4<f32>,
    col2: vec4<f32>,
    col3: vec4<f32>,
}
@group(3) @binding(0)
var<uniform> projection: ProjectionInput;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) pixel: vec2<u32>,
    @location(1) z_value: f32,
    @location(2) @interpolate(flat) resize: u32,
}

// Fragment output with two render targets:
// - location(0): visible color
// - location(1): picking data (pixel_x, pixel_y)
struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) picking: vec2<u32>,
}

@vertex
fn vs_main(data: VertexInput) -> VertexOutput {
    let resize = max(mip_level * 2u, 1u);
    let col = data.index % (image_dims.width) / resize;
    let row = data.index / (image_dims.width) / resize;
    // Map grid coordinates to NDC consistently across the full width/height
    let x = 2.0 * f32(col) / f32(image_dims.width / resize - 1u) - 1.0;
    let y = 1.0 - 2.0 * f32(row) / f32(image_dims.height / resize - 1u);
    let z_value = textureLoad(surface_texture, vec2<u32>(col, row), i32(mip_level));
    let z_clamped = clamp(z_value.x, z_range.min, z_range.max);
    let z = 1.0 - (z_clamped - z_range.min) / (z_range.max - z_range.min);
    let points = vec4<f32>(x, y, z, 1.0);


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
    let world_position = transformation_matrix * points;
    let projected_position = projection_matrix * world_position;

    var out: VertexOutput;
    out.position = projected_position;
    out.pixel = vec2<u32>(col, row);
    out.z_value = z_clamped;
    out.resize = resize;

    return out;
}

@fragment
fn fs_amplitude(in: VertexOutput) -> FragmentOutput {
    let sampled = textureLoad(amplitude_texture, in.pixel * in.resize, 0);
    var out: FragmentOutput;
    out.color = vec4<f32>(1.0 - sampled.r, sampled.r, 0.0, 1.0);
    out.picking = vec2<u32>(in.pixel.x * in.resize, in.pixel.y * in.resize);
    return out;
}

@fragment
fn fs_height(in: VertexOutput) -> FragmentOutput {    
    let overlay_color = textureLoad(overlay_texture, in.pixel * in.resize, 0);
    
    // Calculate base height color
    let depth = (in.z_value - z_range.min) / (z_range.max - z_range.min);
    var base_color = vec4<f32>(depth, depth, depth, 1.0);
    
    // Blend overlay if present (alpha > 0)
    if (overlay_color.a > 0.0) {
        // Alpha blend: result = overlay * alpha + base * (1 - alpha)
        let alpha = overlay_color.a;
        base_color = vec4<f32>(
            overlay_color.rgb * alpha + base_color.rgb * (1.0 - alpha),
            1.0
        );
    }
    
    var out: FragmentOutput;
    out.color = base_color;
    out.picking = vec2<u32>(in.pixel.x * in.resize, in.pixel.y * in.resize);
    return out;
}