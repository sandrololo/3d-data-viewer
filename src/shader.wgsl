struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct TransformationInput {
    @location(1) col0: vec4<f32>,
    @location(2) col1: vec4<f32>,
    @location(3) col2: vec4<f32>,
    @location(4) col3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(vertices: VertexInput, transformation: TransformationInput) -> VertexOutput {
    let matrix = mat4x4<f32>(
        transformation.col0,
        transformation.col1,
        transformation.col2,
        transformation.col3
    );
    let world_position = matrix * vec4<f32>(vertices.position, 1.0);
    
    var out: VertexOutput;
    out.position = world_position;
    return out;
}

const cmin: f32 = 0.3;
const cmax: f32 = 1.0;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = (in.position.z + 1.0) / 2.0 * (cmax - cmin) + cmin;
    let c1 = c * 1.0;
    let c2 = c * 1.0;
    let c3 = c * 1.0;
    return vec4<f32>(c1, c2, c3, 1.0);
}