struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct TransformationInput {
    @location(1) col0: vec4<f32>,
    @location(2) col1: vec4<f32>,
    @location(3) col2: vec4<f32>,
    @location(4) col3: vec4<f32>,
}

struct ProjectionInput{
    @location(5) col0: vec4<f32>,
    @location(6) col1: vec4<f32>,
    @location(7) col2: vec4<f32>,
    @location(8) col3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(vertices: VertexInput, transformation: TransformationInput, projection: ProjectionInput) -> VertexOutput {
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
    let world_position = transformation_matrix * vec4<f32>(vertices.position, 1.0);
    let projected_position = projection_matrix * world_position;

    var out: VertexOutput;
    out.position = projected_position;
    return out;
}

const cmin: f32 = 0.3;
const cmax: f32 = 1.0;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, in.position.z, in.position.z, 1.0);
}