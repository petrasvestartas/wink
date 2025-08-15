#[derive(Clone)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
}

impl Instance {
    // Convert the Instance data into InstanceRaw (no cgmath needed)
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw { model: self.model }
    }

    // Identity instance helper
    pub fn identity() -> Self {
        Self { model: InstanceRaw::identity().model }
    }

    // Create from openmodel::primitives::Xform (column-major f64 -> f32)
    pub fn from_xform(xf: &openmodel::primitives::Xform) -> Self {
        let m = &xf.m;
        Self {
            model: [
                [m[0] as f32, m[1] as f32, m[2] as f32, m[3] as f32],
                [m[4] as f32, m[5] as f32, m[6] as f32, m[7] as f32],
                [m[8] as f32, m[9] as f32, m[10] as f32, m[11] as f32],
                [m[12] as f32, m[13] as f32, m[14] as f32, m[15] as f32],
            ],
        }
    }
}
 


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BatchKind {
    Surface,
    Pipe,
    Sphere,
}

// Since quaternion are not available in shader,
// we need to convert the Instance data into a matrix and store it in a struct called InstanceRaw.
// This is the data that will go into the wgpu::Buffer
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}


impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

    pub fn identity() -> Self {
        Self { model: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ] }
    }
}
 

#[derive(Clone)]
pub struct DrawBatch {
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
    pub instances: Vec<Instance>, // empty => default identity at draw time
    pub kind: BatchKind,
}

pub struct BatchDraw {
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
    pub instance_offset: u32, // into flattened instance array
    pub instance_count: u32,
    pub kind: BatchKind,
}