use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PipeTransform {
    pub transform: [f32; 16], // 4x4 transformation matrix in column-major order
}

impl From<&openmodel::primitives::Xform> for PipeTransform {
    fn from(xf: &openmodel::primitives::Xform) -> Self {
        Self { transform: xf.m }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SphereTransform {
    pub transform: [f32; 16], // 4x4 transformation matrix in column-major order
}

impl From<&openmodel::primitives::Xform> for SphereTransform {
    fn from(xf: &openmodel::primitives::Xform) -> Self {
        Self { transform: xf.m }
    }
}


pub struct GpuGeometryPipeline {
    pub pipe_render_pipeline: wgpu::RenderPipeline,
    pub sphere_render_pipeline: wgpu::RenderPipeline,
    pub data_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuGeometryPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        depth_format: wgpu::TextureFormat,
        msaa_sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU Geometry Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_geometry.wgsl").into()),
        });

        // Create data bind group layout (for line and point data)
        let data_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Lines buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Points buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("GPU Geometry Data Bind Group Layout"),
        });


        // Create render pipelines
        let pipe_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipe Render Pipeline Layout"),
            bind_group_layouts: &[&data_bind_group_layout, camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipe_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipe Render Pipeline"),
            layout: Some(&pipe_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_pipes"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: msaa_sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let sphere_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sphere Render Pipeline Layout"),
            bind_group_layouts: &[&data_bind_group_layout, camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sphere_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sphere Render Pipeline"),
            layout: Some(&sphere_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_spheres"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: msaa_sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            pipe_render_pipeline,
            sphere_render_pipeline,
            data_bind_group_layout,
            camera_bind_group_layout: camera_bind_group_layout.clone(),
        }
    }

    pub fn update_data(&self, device: &wgpu::Device, pipes: Vec<PipeTransform>, spheres: Vec<SphereTransform>) -> wgpu::BindGroup {
        // Create buffers with minimum size to avoid zero-sized buffers
        let pipes_size = if pipes.is_empty() { std::mem::size_of::<PipeTransform>() } else { pipes.len() * std::mem::size_of::<PipeTransform>() };
        let spheres_size = if spheres.is_empty() { std::mem::size_of::<SphereTransform>() } else { spheres.len() * std::mem::size_of::<SphereTransform>() };
        
        let pipes_buffer = if pipes.is_empty() {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Pipes Buffer (Empty)"),
                size: pipes_size as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pipes Buffer"),
                contents: bytemuck::cast_slice(&pipes),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };

        let spheres_buffer = if spheres.is_empty() {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Spheres Buffer (Empty)"),
                size: spheres_size as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Spheres Buffer"),
                contents: bytemuck::cast_slice(&spheres),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pipes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: spheres_buffer.as_entire_binding(),
                },
            ],
            label: Some("GPU Geometry Data Bind Group"),
        });

        bind_group
    }


    pub fn render_pipes(
        &self,
        render_pass: &mut wgpu::RenderPass,
        data_bind_group: &wgpu::BindGroup,
        camera_bind_group: &wgpu::BindGroup,
        num_pipes: u32,
    ) {
        if num_pipes > 0 {
            render_pass.set_pipeline(&self.pipe_render_pipeline);
            render_pass.set_bind_group(0, data_bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            // 48 vertices per pipe (16 triangles * 3 vertices)
            let total_vertices = num_pipes * 48;
            render_pass.draw(0..total_vertices, 0..1);
        }
    }

    pub fn render_spheres(
        &self,
        render_pass: &mut wgpu::RenderPass,
        data_bind_group: &wgpu::BindGroup,
        camera_bind_group: &wgpu::BindGroup,
        num_spheres: u32,
    ) {
        if num_spheres > 0 {
            render_pass.set_pipeline(&self.sphere_render_pipeline);
            render_pass.set_bind_group(0, data_bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            // 144 vertices per sphere (24 quads * 6 vertices)
            let total_vertices = num_spheres * 144;
            render_pass.draw(0..total_vertices, 0..1);
        }
    }
}
