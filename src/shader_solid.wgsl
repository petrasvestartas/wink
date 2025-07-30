// Vertex shader: https://www.w3.org/TR/WGSL/
// WGSL (WebGPU Shading Language) is the shader language for WebGPU

// Define a struct to store the output of the vertex shader.
// This is the value we want to use as clip coordinates for rasterization.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, // Built-in position attribute for GPU pipeline
};

// VERTEX SHADER: Runs once per vertex
// This shader creates a triangle without using vertex buffers (procedural generation)
@vertex
fn vs_main(
    // Built-in vertex index (0, 1, 2 for a triangle)
    // GPU automatically provides this value for each vertex
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Procedurally generate triangle vertices based on vertex index
    // This creates a triangle without needing vertex buffer data
    // Index 0: x = 0.5,  y = -0.5  (bottom right)
    // Index 1: x = -0.5, y = -0.5  (bottom left) 
    // Index 2: x = 0.5,  y = 0.5   (top right)
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    
    // Set the clip space position (x, y, z, w)
    // z = 0.0 (flat triangle), w = 1.0 (perspective division)
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// FRAGMENT SHADER: Runs once per pixel inside the triangle
// Determines the final color of each pixel

// Set the current fragment (pixel) to brown color
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Return RGBA color: (red=0.3, green=0.2, blue=0.1, alpha=1.0)
    // This creates a brown color for the entire triangle
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}