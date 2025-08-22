// Simplified point cloud vertex structure for instanced rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointCloudInstance {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub size: f32,
}

impl PointCloudInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PointCloudInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Size
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ]
        }
    }
}

// Quad vertex for instanced rendering (shared geometry)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
}

impl QuadVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ]
        }
    }

    // Create quad vertices for instanced rendering (6 vertices for 2 triangles)
    pub const VERTICES: &'static [QuadVertex] = &[
        QuadVertex { position: [-1.0, -1.0] }, // Bottom-left
        QuadVertex { position: [ 1.0, -1.0] }, // Bottom-right
        QuadVertex { position: [-1.0,  1.0] }, // Top-left
        QuadVertex { position: [ 1.0, -1.0] }, // Bottom-right
        QuadVertex { position: [ 1.0,  1.0] }, // Top-right
        QuadVertex { position: [-1.0,  1.0] }, // Top-left
    ];
}

pub fn create(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    depth_format: wgpu::TextureFormat,
    msaa_sample_count: u32,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Point Cloud Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader_pointcloud.wgsl").into()),
    });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Point Cloud Render Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Point Cloud Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        cache: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[QuadVertex::desc(), PointCloudInstance::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // No culling for billboards
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: depth_format,
            depth_write_enabled: true, // Write to depth buffer for proper occlusion
            depth_compare: wgpu::CompareFunction::Less, // Normal depth testing
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: msaa_sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}
