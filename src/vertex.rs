use bytemuck::{Pod, Zeroable};


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, // How wide is a vertex in bytes 24
            step_mode: wgpu::VertexStepMode::Vertex, // Vertex data or pre-instance data
            attributes: &[  // mapping to the struct attributes
                wgpu::VertexAttribute {
                    offset: 0, // offset in bytes until the attribute starts
                    shader_location: 0, // location in the shader 0 - position, 1 - color
                    format: wgpu::VertexFormat::Float32x3, // same as vec3<f32>
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ]
        }
    }
}

