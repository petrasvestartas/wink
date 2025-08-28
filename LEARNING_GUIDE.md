# Wink Project Learning Guide

A step-by-step guide to understanding the Wink project - a Rust-based 3D graphics viewer using wgpu and winit.

## Project Overview

**Wink** is a cross-platform 3D graphics application that runs natively on desktop and in web browsers via WebAssembly. It's built with:
- **Rust** - Core language
- **wgpu** - Modern graphics API (WebGPU/Vulkan/Metal/DX12)
- **winit** - Cross-platform windowing
- **cgmath** - 3D math library
- **VuePress** - Documentation and web demo

## Learning Path

### Phase 1: Foundation (Start Here)
**Goal**: Understand basic structure and entry points

#### 1.1 Entry Points
- **`src/main.rs`** - Native desktop entry point
- **`src/lib.rs`** - Main application logic and WASM entry
- **`Cargo.toml`** - Dependencies and build configuration

#### 1.2 Core Data Structures
- **`src/vertex.rs`** - Basic 3D vertex representation
- **`src/instance.rs`** - Object instancing for efficient rendering
- **`src/timing.rs`** - Cross-platform timing utilities

**Learning Exercise**: Read these files to understand how the app starts and basic data flow.

### Phase 2: Graphics Foundation
**Goal**: Understand rendering pipeline basics

#### 2.1 Camera System
- **`src/camera.rs`** - 3D camera with perspective/orthographic projection
  - Quaternion-based rotation
  - Smooth camera controls
  - View/projection matrix calculations

#### 2.2 Buffer Management
- **`src/buffer_factory.rs`** - GPU buffer creation utilities
  - Vertex buffers
  - Index buffers
  - Uniform buffers
  - Instance buffers

#### 2.3 Scene Management
- **`src/scene_bounds.rs`** - Automatic scene framing
- **`src/error_handling.rs`** - Robust error handling

**Learning Exercise**: Study the camera system - it's the heart of 3D interaction.

### Phase 3: Rendering Pipelines
**Goal**: Understand different rendering modes

#### 3.1 Basic Pipelines
- **`src/shader_solid_pipeline.rs`** - Single color rendering
- **`src/shader_color_pipeline.rs`** - Per-vertex color rendering

#### 3.2 Advanced Pipelines
- **`src/shader_lights_pipeline.rs`** - Lit surface rendering with normals
- **`src/shader_pointcloud_pipeline.rs`** - Instanced point cloud rendering
- **`src/shader_primitives_pipeline.rs`** - GPU-generated pipes and spheres
- **`src/shader_arrow_pipeline.rs`** - Arrow/vector visualization

#### 3.3 Shader Files (WGSL)
- **`src/shader_*.wgsl`** - GPU shader programs
  - Vertex shaders (geometry transformation)
  - Fragment shaders (pixel coloring)

**Learning Exercise**: Start with solid pipeline, then move to color pipeline to see the progression.

### Phase 4: Geometry System
**Goal**: Understand the OpenModel geometry library

#### 4.1 Core Primitives (`src/openmodel/src/primitives/`)
- **`point.rs`** - 3D points
- **`vector.rs`** - 3D vectors
- **`color.rs`** - Color representation
- **`xform.rs`** - 3D transformations

#### 4.2 Geometry Types (`src/openmodel/src/geometry/`)
- **`pointcloud.rs`** - Point collections
- **`line.rs`** - Line segments
- **`linecloud.rs`** - Line collections
- **`mesh.rs`** - Triangle meshes
- **`pipe.rs`** - Cylindrical pipes
- **`arrow.rs`** - Arrow vectors
- **`plane.rs`** - Planar surfaces
- **`pline.rs`** - Polylines

#### 4.3 Data Management
- **`src/openmodel/src/common/`** - JSON serialization and data structures
- **`src/geometry_loader.rs`** - Loading geometry from files/HTTP

**Learning Exercise**: Look at the test files in `src/openmodel/tests/` to see geometry usage examples.

### Phase 5: Advanced Features
**Goal**: Understand performance and platform features

#### 5.1 GPU Compute
- **`src/shader_primitives_pipeline.rs`** - GPU-based geometry generation
  - Compute shaders for pipes and spheres
  - Instance-based rendering
  - Dynamic geometry scaling

#### 5.2 Platform Abstraction
- **Native vs WASM builds** - Conditional compilation
- **Cross-platform input handling**
- **File system vs HTTP loading**

#### 5.3 Web Integration
- **`docs/`** - VuePress documentation site
- **`pkg/`** - WASM build output
- **WebGL/WebGPU compatibility**

### Phase 6: Application Logic
**Goal**: Understand the complete application flow

#### 6.1 Main Application Loop (`src/lib.rs`)
- **State management** - The main `State` struct
- **Event handling** - Mouse, keyboard, window events
- **Render loop** - Frame-by-frame rendering
- **Geometry updates** - Dynamic scene changes

#### 6.2 Performance Features
- **Instanced rendering** - Efficient object duplication
- **MSAA** - Multi-sample anti-aliasing
- **Depth testing** - Proper 3D occlusion
- **Frustum culling** - Render only visible objects

## Key Concepts to Master

### 1. **wgpu Pipeline Architecture**
- Render pipelines define how geometry becomes pixels
- Bind groups connect data (buffers, textures) to shaders
- Command encoders record GPU operations

### 2. **Instance-Based Rendering**
- One geometry definition, many transformations
- Efficient for rendering many similar objects
- GPU processes all instances in parallel

### 3. **Shader Programming (WGSL)**
- Vertex shaders transform 3D positions to screen space
- Fragment shaders determine pixel colors
- Compute shaders for general GPU computation

### 4. **Cross-Platform Considerations**
- Different GPU backends (Vulkan, Metal, DX12, WebGL)
- Platform-specific limits and features
- WASM vs native compilation differences

## Debugging and Development Tips

### 1. **Start Simple**
- Begin with the solid color pipeline
- Add one feature at a time
- Use the existing test files as examples

### 2. **Visual Debugging**
- Toggle between pipeline modes (keys 1-4)
- Use orthographic camera for precise measurements
- Adjust pipe radius for better visibility

### 3. **Performance Monitoring**
- Watch frame times in debug output
- Monitor GPU memory usage
- Profile geometry loading times

### 4. **Common Issues**
- **Black screen**: Check camera position and scene bounds
- **Geometry not visible**: Verify vertex data and transformations
- **WASM crashes**: Check WebGL limits and buffer sizes

## Recommended Learning Order

1. **Week 1**: Read Phase 1 & 2 files, understand basic structure
2. **Week 2**: Study one rendering pipeline in detail (start with solid)
3. **Week 3**: Explore geometry system and OpenModel library
4. **Week 4**: Understand the main application loop and state management
5. **Week 5**: Dive into advanced features like GPU compute
6. **Week 6**: Study WASM integration and web deployment

## Next Steps

After completing this guide:
- **Experiment** with the existing codebase
- **Add new geometry types** to OpenModel
- **Create custom shaders** for special effects
- **Optimize performance** for larger datasets
- **Extend web interface** with new controls

## Resources

- **wgpu Tutorial**: https://sotrh.github.io/learn-wgpu/
- **WGSL Specification**: https://www.w3.org/TR/WGSL/
- **Rust Graphics**: https://github.com/gfx-rs/wgpu
- **WebAssembly**: https://rustwasm.github.io/docs/book/

---

*This guide reflects the current state of the Wink project. As the codebase evolves, refer to the latest source code for the most accurate information.*
