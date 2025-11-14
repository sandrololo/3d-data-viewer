struct VertexInput {
    @location(0) z_values: f32,
    @location(1) vertex_index: u32,
}

struct ImageSize{
    @location(2) width: u32,
    @location(3) height: u32,
}

struct ZValueRange{
    @location(4) z_min: f32,
    @location(5) z_max: f32,
}

struct TransformationInput {
    @location(6) col0: vec4<f32>,
    @location(7) col1: vec4<f32>,
    @location(8) col2: vec4<f32>,
    @location(9) col3: vec4<f32>,
}

struct ProjectionInput{
    @location(10) col0: vec4<f32>,
    @location(11) col1: vec4<f32>,
    @location(12) col2: vec4<f32>,
    @location(13) col3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
}

@vertex
fn vs_main(data: VertexInput, image_size: ImageSize, z_range: ZValueRange, transformation: TransformationInput, projection: ProjectionInput) -> VertexOutput {
    let idx = data.vertex_index;
    let col = idx % image_size.width;
    let row = idx / image_size.width;
    // Map grid coordinates to NDC consistently across the full width/height
    let x = -1.0 + 2.0 * f32(col) / f32(image_size.width - 1u);
    let y = -1.0 + 2.0 * f32(row) / f32(image_size.height - 1u);
    let z = -1.0 + (data.z_values - z_range.z_min) / (z_range.z_max - z_range.z_min) * (2.0);
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
        f32(col) / f32(image_size.width - 1u),
        f32(row) / f32(image_size.height - 1u)
    );
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return vec4<f32>(1.0 - sampled.r, sampled.r, 0.0, 1.0);
}