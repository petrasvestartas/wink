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
