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

struct AxesVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct AxesVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct AxesFragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) picking: vec2<u32>,
}

@vertex
fn vs_axes(in: AxesVertexInput) -> AxesVertexOutput {
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

    let world_position = transformation_matrix * vec4<f32>(in.position, 1.0);
    let projected_position = projection_matrix * world_position;

    var out: AxesVertexOutput;
    out.position = projected_position;
    out.color = in.color;
    return out;
}

@fragment
fn fs_axes(in: AxesVertexOutput) -> AxesFragmentOutput {
    var out: AxesFragmentOutput;
    out.color = in.color;
    out.picking = vec2<u32>(0xFFFFFFFFu, 0xFFFFFFFFu);
    return out;
}
