use std::collections::HashMap;

/// Font size in pixels used for rasterisation.
const FONT_PX: f32 = 32.0;

/// Characters needed for axis labels.
const GLYPHS: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', '.', ' ',
];

/// Metrics for a single rasterised glyph.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphInfo {
    /// UV rect in the atlas: (u_min, v_min, u_max, v_max).
    pub uv: [f32; 4],
    /// Size of the glyph bitmap in pixels.
    pub width: f32,
    pub height: f32,
    /// Offset from the pen position to the top-left of the bitmap (pixels).
    pub offset_x: f32,
    pub offset_y: f32,
    /// Horizontal advance after this glyph (pixels).
    pub advance: f32,
}

#[allow(dead_code)]
pub(crate) struct FontAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    glyphs: HashMap<char, GlyphInfo>,
    /// Line height in pixels (ascent - descent).
    pub line_height: f32,
    /// Ascent in pixels (baseline to top).
    pub ascent: f32,
}

impl FontAtlas {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let font_data = include_bytes!("../DejaVuSansMono.ttf");
        let font = fontdue::Font::from_bytes(font_data as &[u8], fontdue::FontSettings::default())
            .expect("failed to parse embedded font");

        // Rasterise all glyphs and collect bitmaps.
        let mut rasterised: Vec<(char, fontdue::Metrics, Vec<u8>)> = Vec::new();
        for &ch in GLYPHS {
            let (metrics, bitmap) = font.rasterize(ch, FONT_PX);
            rasterised.push((ch, metrics, bitmap));
        }

        // Pack glyphs into a single-row atlas with 1px padding.
        let padding = 2u32;
        let atlas_height = rasterised
            .iter()
            .map(|(_, m, _)| m.height as u32)
            .max()
            .unwrap_or(1)
            + padding * 2;
        let atlas_width: u32 = rasterised
            .iter()
            .map(|(_, m, _)| m.width as u32 + padding)
            .sum::<u32>()
            + padding;

        // Build the atlas bitmap (single channel, R8).
        let mut atlas_data = vec![0u8; (atlas_width * atlas_height) as usize];
        let mut cursor_x = padding;
        let mut glyphs = HashMap::new();

        let horizontal_metrics = font.horizontal_line_metrics(FONT_PX).unwrap();
        let ascent = horizontal_metrics.ascent;
        let line_height = horizontal_metrics.ascent - horizontal_metrics.descent;

        for (ch, metrics, bitmap) in &rasterised {
            let gw = metrics.width as u32;
            let gh = metrics.height as u32;

            // Copy bitmap into atlas.
            for row in 0..gh {
                for col in 0..gw {
                    let src = (row * gw + col) as usize;
                    let dst = ((padding + row) * atlas_width + cursor_x + col) as usize;
                    atlas_data[dst] = bitmap[src];
                }
            }

            let u_min = cursor_x as f32 / atlas_width as f32;
            let v_min = padding as f32 / atlas_height as f32;
            let u_max = (cursor_x + gw) as f32 / atlas_width as f32;
            let v_max = (padding + gh) as f32 / atlas_height as f32;

            glyphs.insert(
                *ch,
                GlyphInfo {
                    uv: [u_min, v_min, u_max, v_max],
                    width: gw as f32,
                    height: gh as f32,
                    offset_x: metrics.xmin as f32,
                    offset_y: metrics.ymin as f32,
                    advance: metrics.advance_width,
                },
            );

            cursor_x += gw + padding;
        }

        // Create GPU texture.
        let texture_size = wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("font_atlas_texture"),
            size: texture_size,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width),
                rows_per_image: Some(atlas_height),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("font_atlas_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("font_atlas_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("font_atlas_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group_layout,
            bind_group,
            glyphs,
            line_height,
            ascent,
        }
    }

    /// Look up glyph metrics for a character (falls back to space).
    pub(crate) fn glyph(&self, ch: char) -> &GlyphInfo {
        self.glyphs
            .get(&ch)
            .unwrap_or_else(|| self.glyphs.get(&' ').unwrap())
    }

    /// Compute the total pixel width of a string.
    pub(crate) fn text_width(&self, text: &str) -> f32 {
        text.chars().map(|ch| self.glyph(ch).advance).sum()
    }
}
