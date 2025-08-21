use wgpu::util::DeviceExt;
use bytemuck::Pod;

/// Buffer creation utilities to reduce repetitive patterns
pub struct BufferUtils;

impl BufferUtils {
    /// Create a GPU buffer with consistent pattern
    pub fn create_buffer<T: Pod>(
        device: &wgpu::Device,
        label: &str,
        data: &[T],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage,
        })
    }

    /// Create vertex buffer
    pub fn create_vertex_buffer<T: Pod>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer {
        Self::create_buffer(device, "Vertex Buffer", data, wgpu::BufferUsages::VERTEX)
    }

    /// Create index buffer
    pub fn create_index_buffer<T: Pod>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer {
        Self::create_buffer(device, "Index Buffer", data, wgpu::BufferUsages::INDEX)
    }

    /// Create instance buffer
    pub fn create_instance_buffer<T: Pod>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer {
        Self::create_buffer(
            device,
            "Instance Buffer",
            data,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        )
    }

    /// Create uniform buffer
    pub fn create_uniform_buffer<T: Pod>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer {
        Self::create_buffer(
            device,
            "Uniform Buffer",
            data,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        )
    }

    /// Create point cloud instance buffer
    pub fn create_pointcloud_buffer<T: Pod>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer {
        Self::create_buffer(
            device,
            "Point Cloud Instance Buffer",
            data,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        )
    }

    /// Create point cloud quad buffer
    pub fn create_quad_buffer<T: Pod>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer {
        Self::create_buffer(
            device,
            "Point Cloud Quad Buffer",
            data,
            wgpu::BufferUsages::VERTEX,
        )
    }
}
