struct VertexInput {
    @location(0) z_values: f32,
    @location(1) vertex_index: u32,
}

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
    @location(0) tex_coords: vec2<f32>,
    @location(1) points_z: f32,
}

@vertex
fn vs_main(data: VertexInput) -> VertexOutput {
    let idx = data.vertex_index;
    let col = idx % image_dims.width;
    let row = idx / image_dims.width;
    // Map grid coordinates to NDC consistently across the full width/height
    let x = -1.0 + 2.0 * f32(col) / f32(image_dims.width - 1u);
    let y = -1.0 + 2.0 * f32(row) / f32(image_dims.height - 1u);
    let z = -1.0 + 2.0 * (data.z_values - z_range.min) / (z_range.max - z_range.min);
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
    out.tex_coords = vec2<f32>(
        f32(col) / f32(image_dims.width - 1u),
        f32(row) / f32(image_dims.height - 1u)
    );
    out.points_z = data.z_values;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_overlay: texture_2d<f32>;

@fragment
fn fs_amplitude(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return vec4<f32>(1.0 - sampled.r, sampled.r, 0.0, 1.0);
}

@fragment
fn fs_height(in: VertexOutput) -> @location(0) vec4<f32> {    
    // Calculate pixel coordinates from texture coordinates
    let col = u32(in.tex_coords.x * f32(image_dims.width - 1u) + 0.5);
    let row = u32(in.tex_coords.y * f32(image_dims.height - 1u) + 0.5);
    
    // Sample overlay texture directly using col, row
    let overlay_color = textureLoad(t_overlay, vec2<i32>(i32(col), i32(row)), 0);
    
    // Calculate base height color
    let depth = 0.10 + 0.80 * (in.points_z - z_range.min) / (z_range.max - z_range.min);
    let base_color = vec4<f32>(depth, depth, depth, 1.0);
    
    // Blend overlay if present (alpha > 0)
    if (overlay_color.a > 0.0) {
        // Alpha blend: result = overlay * alpha + base * (1 - alpha)
        let alpha = overlay_color.a;
        return vec4<f32>(
            overlay_color.rgb * alpha + base_color.rgb * (1.0 - alpha),
            1.0
        );
    }
    
    return base_color;
}