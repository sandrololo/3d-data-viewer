use std::borrow::Cow;

use glam::Mat4;
use wgpu::{Device, RenderPass, util::DeviceExt};

use crate::{
    gpu_data::pixel_picker::PixelPicker, interaction::Interaction,
    render::depth_buffer::DepthBuffer, render::font_atlas::FontAtlas,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GridVertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl GridVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GridVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LabelVertex {
    anchor: [f32; 3],    // world-space position that the label is anchored to
    offset_px: [f32; 2], // pixel offset from the anchor point
    uv: [f32; 2],        // (u_min, v_min) of the glyph in the atlas
    color: [f32; 4],
}

impl LabelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LabelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Pixel size of a label glyph on screen.
const LABEL_FONT_SCALE: f32 = 0.45;

/// Append six vertices (two triangles) for one glyph quad.
fn push_glyph_quad(
    verts: &mut Vec<LabelVertex>,
    anchor: [f32; 3],
    px_x: f32,
    px_y: f32,
    glyph_w: f32,
    glyph_h: f32,
    uv: [f32; 4], // u_min, v_min, u_max, v_max
    color: [f32; 4],
) {
    let (u0, v0, u1, v1) = (uv[0], uv[1], uv[2], uv[3]);
    let x0 = px_x;
    let y0 = px_y;
    let x1 = px_x + glyph_w;
    let y1 = px_y + glyph_h;

    // Two triangles: top-left, bottom-left, bottom-right, top-left, bottom-right, top-right.
    // Note: screen Y goes down in pixel space but up in NDC. The shader adds
    // offset_px.y to clip.y, so positive offset_px.y moves UP on screen.
    // We want v0 (top of glyph texture) at the higher Y (y1) and v1 at y0.
    let tl = LabelVertex {
        anchor,
        offset_px: [x0, y1],
        uv: [u0, v0],
        color,
    };
    let bl = LabelVertex {
        anchor,
        offset_px: [x0, y0],
        uv: [u0, v1],
        color,
    };
    let br = LabelVertex {
        anchor,
        offset_px: [x1, y0],
        uv: [u1, v1],
        color,
    };
    let tr = LabelVertex {
        anchor,
        offset_px: [x1, y1],
        uv: [u1, v0],
        color,
    };

    verts.push(tl);
    verts.push(bl);
    verts.push(br);
    verts.push(tl);
    verts.push(br);
    verts.push(tr);
}

/// Emit label quads for a text string anchored at a 3-D position.
///
/// `anchor`     – world-space position that the label is attached to.
/// `align`      – (0.0, 0.0) = text starts at anchor; (0.5, 0.5) = centered;
///                (1.0, 0.5) = right-aligned, vertically centered.
/// `offset_px`  – extra pixel offset applied after alignment.
fn push_label(
    verts: &mut Vec<LabelVertex>,
    font: &FontAtlas,
    text: &str,
    anchor: [f32; 3],
    align: (f32, f32),
    offset_px: (f32, f32),
    color: [f32; 4],
) {
    let scale = LABEL_FONT_SCALE;
    let total_w = font.text_width(text) * scale;
    let total_h = font.line_height * scale;

    // Start pen position (pixels relative to anchor).
    let pen_x = offset_px.0 - align.0 * total_w;
    let pen_y = offset_px.1 - align.1 * total_h;

    let mut cx = pen_x;
    for ch in text.chars() {
        let g = font.glyph(ch);
        let gw = g.width * scale;
        let gh = g.height * scale;
        let ox = g.offset_x * scale;
        // fontdue's ymin is the distance from the baseline to the bottom of the glyph
        // (positive means above baseline). We convert to pixel-up coords.
        let oy = g.offset_y * scale;

        if gw > 0.0 && gh > 0.0 {
            push_glyph_quad(verts, anchor, cx + ox, pen_y + oy, gw, gh, g.uv, color);
        }
        cx += g.advance * scale;
    }
}

/// Number of subdivisions per axis on the back walls.
const GRID_DIVISIONS: u32 = 10;

/// Returns tick values: 0, step, 2*step, …, and the final max_pixel value.
fn axis_tick_values(max_pixel: u32, step: u32) -> Vec<u32> {
    let mut values = Vec::new();
    let mut v = 0;
    while v < max_pixel {
        values.push(v);
        v += step;
    }
    if *values.last().unwrap_or(&0) != max_pixel {
        values.push(max_pixel);
    }
    values
}

/// Like `axis_tick_values` but drops the last step value if it is within
/// `min_gap` pixels of `max_pixel`, to avoid overlapping labels near the end.
fn axis_label_values(max_pixel: u32, step: u32, min_gap: u32) -> Vec<u32> {
    let mut values = axis_tick_values(max_pixel, step);
    // The last element is always max_pixel. If the second-to-last is too close, drop it.
    if values.len() >= 2 {
        let penult = values[values.len() - 2];
        if penult > 0 && max_pixel - penult < min_gap {
            values.remove(values.len() - 2);
        }
    }
    values
}

/// Format an f32 value into a short label string.
fn format_z_value(value: f32) -> String {
    let abs = value.abs();
    if abs >= 100.0 {
        format!("{:.0}", value)
    } else if abs >= 10.0 {
        format!("{:.1}", value)
    } else if abs >= 1.0 {
        format!("{:.2}", value)
    } else {
        format!("{:.3}", value)
    }
}

struct GridGeometry {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    z_min: f32,
    z_max: f32,
}

impl GridGeometry {
    fn new(aspect_ratio: f32) -> Self {
        Self {
            x_min: -aspect_ratio,
            x_max: aspect_ratio,
            y_min: -1.0,
            y_max: 1.0,
            z_min: -0.5,
            z_max: 0.5,
        }
    }
}

/// Build grid-line vertices: floor + two back walls
///
/// Coordinate space:
///   X: [-aspect_ratio, +aspect_ratio]
///   Y: [-1, +1]
///   Z: [-0.5, +0.5]  (z_min = visual top / data max, z_max = visual bottom / data min)
///
/// Floor grid at z_max, back walls at X=x_min and Y=y_max.
fn build_grid_lines(g: &GridGeometry, image_width: u32, image_height: u32) -> Vec<GridVertex> {
    let grid_color = [0.22, 0.22, 0.30, 1.0];
    let edge_color = [0.35, 0.35, 0.45, 1.0];
    let n = GRID_DIVISIONS;
    let mut verts = Vec::new();

    /// Emit `n+1` lines that interpolate between two endpoints.
    /// `a` and `b` are the positions at t=0 and t=1 of the varying coordinate;
    /// the other coordinate spans from `start` to `end` for each line.
    fn push_grid_lines(
        verts: &mut Vec<GridVertex>,
        n: u32,
        start: [f32; 3],
        end: [f32; 3],
        start_far: [f32; 3],
        end_far: [f32; 3],
        grid_color: [f32; 4],
        edge_color: [f32; 4],
    ) {
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let c = if i == 0 || i == n {
                edge_color
            } else {
                grid_color
            };
            let lerp = |a: f32, b: f32| a + t * (b - a);
            let p0 = [
                lerp(start[0], end[0]),
                lerp(start[1], end[1]),
                lerp(start[2], end[2]),
            ];
            let p1 = [
                lerp(start_far[0], end_far[0]),
                lerp(start_far[1], end_far[1]),
                lerp(start_far[2], end_far[2]),
            ];
            verts.push(GridVertex {
                position: p0,
                color: c,
            });
            verts.push(GridVertex {
                position: p1,
                color: c,
            });
        }
    }

    // ---- Floor at z_max (XY plane) — lines every 50 pixels ----
    let px_step = 50_u32;

    // Lines parallel to X (constant Y, spanning X) — one per 50px of image_height
    {
        let y_vals = axis_tick_values(image_height, px_step);
        for &py in &y_vals {
            let t = py as f32 / image_height as f32;
            let y = g.y_max - t * (g.y_max - g.y_min);
            let c = if py == 0 || py == image_height {
                edge_color
            } else {
                grid_color
            };
            verts.push(GridVertex {
                position: [g.x_min, y, g.z_max],
                color: c,
            });
            verts.push(GridVertex {
                position: [g.x_max, y, g.z_max],
                color: c,
            });
        }
    }
    // Lines parallel to Y (constant X, spanning Y) — one per 50px of image_width
    {
        let x_vals = axis_tick_values(image_width, px_step);
        for &px in &x_vals {
            let t = px as f32 / image_width as f32;
            let x = g.x_min + t * (g.x_max - g.x_min);
            let c = if px == 0 || px == image_width {
                edge_color
            } else {
                grid_color
            };
            verts.push(GridVertex {
                position: [x, g.y_min, g.z_max],
                color: c,
            });
            verts.push(GridVertex {
                position: [x, g.y_max, g.z_max],
                color: c,
            });
        }
    }

    // ---- Back wall at Y = y_max (XZ plane) ----
    // Horizontal lines (constant Z, spanning X)
    push_grid_lines(
        &mut verts,
        n,
        [g.x_min, g.y_max, g.z_min],
        [g.x_min, g.y_max, g.z_max],
        [g.x_max, g.y_max, g.z_min],
        [g.x_max, g.y_max, g.z_max],
        grid_color,
        edge_color,
    );
    // Vertical lines (constant X, spanning Z) — match floor's 50px X spacing
    {
        let x_vals = axis_tick_values(image_width, px_step);
        for &px in &x_vals {
            let t = px as f32 / image_width as f32;
            let x = g.x_min + t * (g.x_max - g.x_min);
            let c = if px == 0 || px == image_width {
                edge_color
            } else {
                grid_color
            };
            verts.push(GridVertex {
                position: [x, g.y_max, g.z_min],
                color: c,
            });
            verts.push(GridVertex {
                position: [x, g.y_max, g.z_max],
                color: c,
            });
        }
    }

    // ---- Back wall at X = x_min (YZ plane) ----
    // Horizontal lines (constant Z, spanning Y)
    push_grid_lines(
        &mut verts,
        n,
        [g.x_min, g.y_min, g.z_min],
        [g.x_min, g.y_min, g.z_max],
        [g.x_min, g.y_max, g.z_min],
        [g.x_min, g.y_max, g.z_max],
        grid_color,
        edge_color,
    );
    // Vertical lines (constant Y, spanning Z) — match floor's 50px Y spacing
    {
        let y_vals = axis_tick_values(image_height, px_step);
        for &py in &y_vals {
            let t = py as f32 / image_height as f32;
            let y = g.y_max - t * (g.y_max - g.y_min);
            let c = if py == 0 || py == image_height {
                edge_color
            } else {
                grid_color
            };
            verts.push(GridVertex {
                position: [g.x_min, y, g.z_min],
                color: c,
            });
            verts.push(GridVertex {
                position: [g.x_min, y, g.z_max],
                color: c,
            });
        }
    }

    // ---- Prominent labeled-axis lines ----
    let axis_color = [0.7, 0.7, 0.8, 1.0];

    // X axis: floor front edge (y_min, z_max)
    verts.push(GridVertex {
        position: [g.x_min, g.y_min, g.z_max],
        color: axis_color,
    });
    verts.push(GridVertex {
        position: [g.x_max, g.y_min, g.z_max],
        color: axis_color,
    });
    // Y axis: floor right edge (x_max, z_max)
    verts.push(GridVertex {
        position: [g.x_max, g.y_min, g.z_max],
        color: axis_color,
    });
    verts.push(GridVertex {
        position: [g.x_max, g.y_max, g.z_max],
        color: axis_color,
    });
    // Z axis: right-back vertical edge (x_max, y_max)
    verts.push(GridVertex {
        position: [g.x_max, g.y_max, g.z_min],
        color: axis_color,
    });
    verts.push(GridVertex {
        position: [g.x_max, g.y_max, g.z_max],
        color: axis_color,
    });

    // ---- Tick marks ----
    let tick_color = [0.6, 0.6, 0.7, 1.0];
    let tick_len = 0.015_f32;

    /// Emit tick marks at each position in `values`. For each value, `pos_fn`
    /// returns the base position; the tick extends from there by `dir * tick_len`.
    fn push_ticks(
        verts: &mut Vec<GridVertex>,
        values: &[u32],
        max_pixel: u32,
        pos_fn: impl Fn(f32) -> [f32; 3],
        dir: [f32; 3],
        tick_len: f32,
        color: [f32; 4],
    ) {
        for &v in values {
            let t = v as f32 / max_pixel as f32;
            let p = pos_fn(t);
            verts.push(GridVertex { position: p, color });
            verts.push(GridVertex {
                position: [
                    p[0] + dir[0] * tick_len,
                    p[1] + dir[1] * tick_len,
                    p[2] + dir[2] * tick_len,
                ],
                color,
            });
        }
    }

    // X ticks along floor front edge (y_min, z_max)
    if image_width > 0 {
        push_ticks(
            &mut verts,
            &axis_tick_values(image_width, 50),
            image_width,
            |t| [g.x_min + t * (g.x_max - g.x_min), g.y_min, g.z_max],
            [0.0, -1.0, 0.0],
            tick_len,
            tick_color,
        );
    }

    // Y ticks along floor right edge (x_max, z_max), inverted: 0 at y_max
    if image_height > 0 {
        push_ticks(
            &mut verts,
            &axis_tick_values(image_height, 50),
            image_height,
            |t| [g.x_max, g.y_max - t * (g.y_max - g.y_min), g.z_max],
            [1.0, 0.0, 0.0],
            tick_len,
            tick_color,
        );
    }

    // Z ticks along right-back vertical edge (x_max, y_max)
    {
        let z_vals: Vec<u32> = (0..=n).collect();
        push_ticks(
            &mut verts,
            &z_vals,
            n,
            |t| [g.x_max, g.y_max, g.z_min + t * (g.z_max - g.z_min)],
            [1.0, 0.0, 0.0],
            tick_len,
            tick_color,
        );
    }

    verts
}

/// Build billboard label vertices for axis numbers.
fn build_label_verts(
    font: &FontAtlas,
    g: &GridGeometry,
    image_width: u32,
    image_height: u32,
    z_range: (f32, f32),
    x_label_step: u32,
    y_label_step: u32,
    z_label_skip: u32,
) -> Vec<LabelVertex> {
    let mut verts = Vec::new();
    let label_color = [0.7, 0.7, 0.8, 1.0];
    let n = GRID_DIVISIONS;
    let gap_px = 6.0_f32;

    // X axis labels along floor front edge (y_min, z_max) — below the ticks
    // x_label_step: 0 = max only, otherwise normal stepping
    let x_ticks = if x_label_step == 0 {
        vec![image_width]
    } else {
        axis_label_values(image_width, x_label_step, x_label_step / 2)
    };
    for &px in &x_ticks {
        let t = px as f32 / image_width as f32;
        let world_x = g.x_min + t * (g.x_max - g.x_min);
        let anchor = [world_x, g.y_min, g.z_max];
        let text = px.to_string();
        push_label(
            &mut verts,
            font,
            &text,
            anchor,
            (1.0, 1.0), // right-aligned (text extends to the left of the tick)
            (-gap_px, -gap_px),
            label_color,
        );
    }

    // Y axis labels along floor right edge (x_max, z_max), inverted: 0 at y_max
    // y_label_step: 0 = max only, otherwise normal stepping
    let y_ticks = if y_label_step == 0 {
        vec![image_height]
    } else {
        axis_label_values(image_height, y_label_step, y_label_step / 2)
    };
    for &py in &y_ticks {
        let t = py as f32 / image_height as f32;
        let world_y = g.y_max - t * (g.y_max - g.y_min);
        let anchor = [g.x_max, world_y, g.z_max];
        let text = py.to_string();
        push_label(
            &mut verts,
            font,
            &text,
            anchor,
            (0.0, 1.0), // left-aligned (text extends to the right of the tick)
            (gap_px, -gap_px),
            label_color,
        );
    }

    // Z axis labels along right-back vertical edge (x_max, y_max)
    // z_label_skip: 0 = max only (last division), otherwise skip factor
    let (z_data_min, z_data_max) = z_range;
    for i in 0..=n {
        if z_label_skip == 0 {
            // max only: show only the first label (i == 0, which is z_min = data_max)
            if i != 0 {
                continue;
            }
        } else if z_label_skip > 1 && i % z_label_skip != 0 {
            continue;
        }
        let t = i as f32 / n as f32;
        let z = g.z_min + t * (g.z_max - g.z_min);
        let data_value = z_data_max + t * (z_data_min - z_data_max);
        let anchor = [g.x_max, g.y_max, z];
        let text = format_z_value(data_value);
        push_label(
            &mut verts,
            font,
            &text,
            anchor,
            (0.0, 0.5), // left-aligned, vertically centered
            (gap_px, 0.0),
            label_color,
        );
    }

    verts
}

/// Screen-size uniform for the label billboard shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenSizeUniform {
    width: f32,
    height: f32,
}

#[allow(dead_code)]
pub(crate) struct Axes {
    // Grid lines (LineList)
    grid_pipeline: wgpu::RenderPipeline,
    grid_vertex_buffer: wgpu::Buffer,
    grid_vertex_count: u32,

    // Billboard text labels (TriangleList)
    label_pipeline: wgpu::RenderPipeline,
    label_vertex_buffer: wgpu::Buffer,
    label_vertex_count: u32,
    font_atlas: FontAtlas,

    // Screen-size uniform for the label billboard shader
    screen_size_buffer: wgpu::Buffer,
    screen_size_bind_group_layout: wgpu::BindGroupLayout,
    screen_size_bind_group: wgpu::BindGroup,

    // Cached state for rebuilding labels
    image_width: u32,
    image_height: u32,
    aspect_ratio: f32,
    z_range: (f32, f32),
    screen_width: f32,
    screen_height: f32,
}

impl Axes {
    pub(crate) fn new(
        device: &Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        interaction: &Interaction,
    ) -> Self {
        // ---- Grid line pipeline (LineList) ----
        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grid_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../axes_shader.wgsl"))),
        });

        let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid_pipeline_layout"),
            bind_group_layouts: &[
                &interaction.transformation.bind_group_layout,
                &interaction.projection.bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let color_targets = [
            Some(wgpu::ColorTargetState {
                format: surface_format.add_srgb_suffix(),
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: PixelPicker::PICKING_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
        ];

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid_pipeline"),
            layout: Some(&grid_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &grid_shader,
                entry_point: Some("vs_axes"),
                buffers: &[GridVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &grid_shader,
                entry_point: Some("fs_axes"),
                compilation_options: Default::default(),
                targets: &color_targets,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(DepthBuffer::depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---- Font atlas ----
        let font_atlas = FontAtlas::new(device, queue);

        // ---- Screen-size uniform ----
        let screen_size_data = ScreenSizeUniform {
            width: 800.0,
            height: 600.0,
        };
        let screen_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen_size_buffer"),
            contents: bytemuck::bytes_of(&screen_size_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let screen_size_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("screen_size_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let screen_size_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen_size_bg"),
            layout: &screen_size_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_size_buffer.as_entire_binding(),
            }],
        });

        // ---- Label pipeline (TriangleList, billboard) ----
        let label_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("label_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../label_shader.wgsl"))),
        });

        let label_color_targets = [
            Some(wgpu::ColorTargetState {
                format: surface_format.add_srgb_suffix(),
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: PixelPicker::PICKING_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
        ];

        let label_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("label_pipeline_layout"),
                bind_group_layouts: &[
                    &interaction.transformation.bind_group_layout, // group 0
                    &interaction.projection.bind_group_layout,     // group 1
                    &screen_size_bind_group_layout,                // group 2
                    &font_atlas.bind_group_layout,                 // group 3
                ],
                push_constant_ranges: &[],
            });

        // Depth test: read (so labels are occluded by the image) but do not
        // write (so labels don't occlude each other or the grid).
        let label_depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let label_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("label_pipeline"),
            layout: Some(&label_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &label_shader,
                entry_point: Some("vs_label"),
                buffers: &[LabelVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &label_shader,
                entry_point: Some("fs_label"),
                compilation_options: Default::default(),
                targets: &label_color_targets,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(label_depth_stencil),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---- Initial (empty) geometry ----
        let g = GridGeometry::new(1.0);
        let grid_verts = build_grid_lines(&g, 0, 0);
        let grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: bytemuck::cast_slice(&grid_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Empty label buffer (no labels until image is loaded).
        let label_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("label_vertex_buffer"),
            contents: &[0u8; std::mem::size_of::<LabelVertex>() * 6],
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            grid_pipeline,
            grid_vertex_buffer,
            grid_vertex_count: grid_verts.len() as u32,

            label_pipeline,
            label_vertex_buffer,
            label_vertex_count: 0,
            font_atlas,

            screen_size_buffer,
            screen_size_bind_group_layout,
            screen_size_bind_group,

            image_width: 0,
            image_height: 0,
            aspect_ratio: 1.0,
            z_range: (0.0, 0.0),
            screen_width: 800.0,
            screen_height: 600.0,
        }
    }

    /// Update the screen-size uniform (call on window resize).
    pub(crate) fn update_screen_size(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.screen_width = width.max(1) as f32;
        self.screen_height = height.max(1) as f32;
        let data = ScreenSizeUniform {
            width: self.screen_width,
            height: self.screen_height,
        };
        queue.write_buffer(&self.screen_size_buffer, 0, bytemuck::bytes_of(&data));
    }

    /// Rebuild the grid to match the loaded image dimensions.
    pub(crate) fn update_grid(
        &mut self,
        device: &wgpu::Device,
        image_width: u32,
        image_height: u32,
        z_range: (f32, f32),
    ) {
        self.image_width = image_width;
        self.image_height = image_height;
        self.aspect_ratio = image_width as f32 / image_height as f32;
        self.z_range = z_range;
        self.rebuild_geometry(device);
    }

    /// Rebuild the grid with a new z-range, keeping the current image dimensions.
    pub(crate) fn update_z_range(&mut self, device: &wgpu::Device, z_range: (f32, f32)) {
        self.z_range = z_range;
        if self.image_width > 0 && self.image_height > 0 {
            self.rebuild_geometry(device);
        }
    }

    fn rebuild_geometry(&mut self, device: &wgpu::Device) {
        let g = GridGeometry::new(self.aspect_ratio);

        // Grid lines
        let grid_verts = build_grid_lines(&g, self.image_width, self.image_height);
        self.grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: bytemuck::cast_slice(&grid_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.grid_vertex_count = grid_verts.len() as u32;

        // Labels will be rebuilt in update_labels with view-dependent density
        self.label_vertex_count = 0;
    }

    /// Rebuild label vertex buffer based on projected axis lengths.
    /// Call this every frame (or when the view changes) before drawing.
    ///
    /// `mvp` is the current model-view-projection matrix used for rendering, needed to
    /// project world-space coordinates to screen-space for label placement.
    pub(crate) fn update_labels(&mut self, device: &wgpu::Device, mvp: Mat4) {
        if self.image_width == 0 || self.image_height == 0 {
            return;
        }

        let g = GridGeometry::new(self.aspect_ratio);
        let sw = self.screen_width;
        let sh = self.screen_height;

        // Project a world-space point to screen pixels.
        let project = |x: f32, y: f32, z: f32| -> (f32, f32) {
            let clip = mvp.mul_vec4(glam::Vec4::new(x, y, z, 1.0));
            let ndc_x = clip.x / clip.w;
            let ndc_y = clip.y / clip.w;
            ((ndc_x * 0.5 + 0.5) * sw, (ndc_y * 0.5 + 0.5) * sh)
        };

        // Screen-space length of each axis in pixels.
        let axis_screen_len = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32| -> f32 {
            let (sx0, sy0) = project(x0, y0, z0);
            let (sx1, sy1) = project(x1, y1, z1);
            ((sx1 - sx0).powi(2) + (sy1 - sy0).powi(2)).sqrt()
        };

        let x_len = axis_screen_len(g.x_min, g.y_min, g.z_max, g.x_max, g.y_min, g.z_max);
        let y_len = axis_screen_len(g.x_max, g.y_min, g.z_max, g.x_max, g.y_max, g.z_max);
        let z_len = axis_screen_len(g.x_max, g.y_max, g.z_min, g.x_max, g.y_max, g.z_max);

        // Choose label step based on screen-space axis length.
        // <40px → max only (step=0), 40–80px → min+max only, >80px → computed density.
        fn pick_xy_step(screen_px: f32, max_pixel: u32) -> u32 {
            if screen_px < 40.0 {
                return 0; // max only
            }
            if screen_px < 80.0 {
                return max_pixel; // min + max only (step >= max → only 0 and max)
            }
            let desired_labels = (screen_px / 80.0).max(2.0);
            let ideal_step = max_pixel as f32 / desired_labels;
            let steps = [50, 100, 200, 500, 1000];
            *steps
                .iter()
                .min_by_key(|&&s| ((s as f32) - ideal_step).abs() as u32)
                .unwrap_or(&200)
        }

        // Z axis: <40px → max only (skip=0), 40–80px → min+max (skip=n), >80px → computed.
        fn pick_z_skip(screen_px: f32) -> u32 {
            if screen_px < 40.0 {
                return 0; // max only
            }
            if screen_px < 80.0 {
                return GRID_DIVISIONS; // min + max only
            }
            let desired_labels = (screen_px / 80.0).max(2.0);
            let n = GRID_DIVISIONS as f32;
            let ideal_skip = (n / desired_labels).max(1.0);
            let skips = [1u32, 2, 5, 10];
            *skips
                .iter()
                .min_by_key(|&&s| ((s as f32) - ideal_skip).abs() as u32)
                .unwrap_or(&2)
        }

        let x_label_step = pick_xy_step(x_len, self.image_width);
        let y_label_step = pick_xy_step(y_len, self.image_height);
        let z_label_skip = pick_z_skip(z_len);

        let label_verts = build_label_verts(
            &self.font_atlas,
            &g,
            self.image_width,
            self.image_height,
            self.z_range,
            x_label_step,
            y_label_step,
            z_label_skip,
        );
        self.label_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("label_vertex_buffer"),
            contents: bytemuck::cast_slice(&label_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.label_vertex_count = label_verts.len() as u32;
    }

    pub(crate) fn draw<'a>(
        &'a self,
        renderpass: &mut RenderPass<'a>,
        interaction: &'a Interaction,
    ) {
        // 1. Draw grid lines
        renderpass.set_pipeline(&self.grid_pipeline);
        renderpass.set_bind_group(0, &interaction.transformation.bind_group, &[]);
        renderpass.set_bind_group(1, &interaction.projection.bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
        renderpass.draw(0..self.grid_vertex_count, 0..1);

        // 2. Draw billboard text labels
        if self.label_vertex_count > 0 {
            renderpass.set_pipeline(&self.label_pipeline);
            renderpass.set_bind_group(0, &interaction.transformation.bind_group, &[]);
            renderpass.set_bind_group(1, &interaction.projection.bind_group, &[]);
            renderpass.set_bind_group(2, &self.screen_size_bind_group, &[]);
            renderpass.set_bind_group(3, &self.font_atlas.bind_group, &[]);
            renderpass.set_vertex_buffer(0, self.label_vertex_buffer.slice(..));
            renderpass.draw(0..self.label_vertex_count, 0..1);
        }
    }
}
