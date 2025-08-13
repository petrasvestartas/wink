// Placeholder module for the upcoming pipe-specific pipeline.
// This keeps the module structure stable while we implement the screen-space radius logic.

#[allow(dead_code)]
pub fn create(
    _device: &wgpu::Device,
    _config: &wgpu::SurfaceConfiguration,
    _camera_bind_group_layout: &wgpu::BindGroupLayout,
    _depth_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    unimplemented!("shader_pipe_pipeline::create is not implemented yet");
}
