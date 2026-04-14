struct VertexInput {
    @location(0) index: u32,
}
@group(0) @binding(0)
var topology_data: texture_2d<f32>;
@group(0) @binding(1)
var texture_image: texture_2d<u32>;
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

struct TextureImageRange{
    start: u32,
    end: u32,
}
@group(1) @binding(2)
var<uniform> texture_image_range: TextureImageRange;

@group(1) @binding(3)
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
    @location(2) @interpolate(linear) light_intensity: f32,
    @location(3) @interpolate(flat) resize: u32,
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
    let resize = u32(exp2(f32(mip_level)));
    let col = data.index % image_dims.width;
    let row = data.index / image_dims.width;

    let img_aspect_ratio = f32(image_dims.width) / f32(image_dims.height);
    // Map grid coordinates to NDC, preserving image aspect ratio and keeping the mesh centred
    let x = img_aspect_ratio * (2.0 * f32(col) / f32(image_dims.width - 1u) - 1.0);
    let y = 1.0 - 2.0 * f32(row) / f32(image_dims.height - 1u);

    let z_value = textureLoad(topology_data, vec2<u32>(col, row) * resize, 0);
    let z_clamped = clamp(z_value.x, z_range.min, z_range.max);

    let light = normalize(vec3(-1.0, 1.0, 1.0));
    let z_up = textureLoad(topology_data, vec2<u32>(col, max(row, 1u) - 1u) * resize, 0).x;
    let z_down = textureLoad(topology_data, vec2<u32>(col, min(row + 1u, image_dims.height - 1u)) * resize, 0).x;
    let z_left = textureLoad(topology_data, vec2<u32>(max(col, 1u) - 1u, row) * resize, 0).x;
    let z_right = textureLoad(topology_data, vec2<u32>(min(col + 1u, image_dims.width - 1u), row) * resize, 0).x;

    let tangent_x = normalize(vec3(2.0, 0.0, (z_right - z_left) * (z_range.max - z_range.min)));
    let tangent_y = normalize(vec3(0.0, 2.0, (z_down - z_up) * (z_range.max - z_range.min)));
    let normal = normalize(cross(tangent_y, tangent_x));


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
    out.light_intensity = max(dot(light, normal), 0.5);
    out.resize = resize;

    return out;
}

@fragment
fn fs_texture(in: VertexOutput) -> FragmentOutput {
    let sampled = textureLoad(texture_image, in.pixel * in.resize, 0);
    let range = f32(texture_image_range.end - texture_image_range.start);
    let red = 1.0 - f32(sampled.r - texture_image_range.start) / range;
    let green = f32(sampled.r - texture_image_range.start) / range;
    var out: FragmentOutput;
    out.color = vec4<f32>(red, green, 0.0, 1.0) * in.light_intensity;
    out.picking = vec2<u32>(in.pixel.x * in.resize, in.pixel.y * in.resize);
    return out;
}

@fragment
fn fs_height(in: VertexOutput) -> FragmentOutput {    
    let overlay_color = textureLoad(overlay_texture, in.pixel * in.resize, 0);
    
    // Calculate base height color
    let depth = (in.z_value - z_range.min) / (z_range.max - z_range.min);
    var color = vec4<f32>(depth, depth, depth, 1.0);
    
    // Blend overlay if present (alpha > 0)
    if (overlay_color.a > 0.0) {
        // Alpha blend: result = overlay * alpha + base * (1 - alpha)
        let alpha = overlay_color.a;
        color = vec4<f32>(
            overlay_color.rgb * alpha + color.rgb * (1.0 - alpha),
            1.0
        );
    }
    
    var out: FragmentOutput;
    out.color = color * in.light_intensity;
    out.picking = vec2<u32>(in.pixel.x * in.resize, in.pixel.y * in.resize);
    return out;
}

@fragment
fn fs_turbo_colormap(in: VertexOutput) -> FragmentOutput {
    let depth = (in.z_value - z_range.min) / (z_range.max - z_range.min);
    var out: FragmentOutput;
    out.picking = vec2<u32>(in.pixel.x * in.resize, in.pixel.y * in.resize);
    out.color = vec4<f32>(turbo_colormap(depth) * in.light_intensity, 1.0);
    return out;
}

// Turbo colormap implementation based https://gist.github.com/mikhailov-work/0d177465a8151eb6ede1768d51d476c7
fn turbo_colormap(x: f32) -> vec3<f32> {
    let t = clamp(x, 0.0, 1.0);

    let kRedVec4 = vec4(0.13572138, 4.61539260, -42.66032258, 132.13108234);
    let kGreenVec4 = vec4(0.09140261, 2.19418839, 4.84296658, -14.18503333);
    let kBlueVec4 = vec4(0.10667330, 12.64194608, -60.58204836, 110.36276771);
    let kRedVec2 = vec2(-152.94239396, 59.28637943);
    let kGreenVec2 = vec2(4.27729857, 2.82956604);
    let kBlueVec2 = vec2(-89.90310912, 27.34824973);
  
    let v4 = vec4( 1.0, x, x * x, x * x * x);
    let v2 = v4.zw * v4.z;
    return vec3(
        dot(v4, kRedVec4)   + dot(v2, kRedVec2),
        dot(v4, kGreenVec4) + dot(v2, kGreenVec2),
        dot(v4, kBlueVec4)  + dot(v2, kBlueVec2)
    );
}