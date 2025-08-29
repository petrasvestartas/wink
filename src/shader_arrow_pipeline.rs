use openmodel::geometry::Arrow;
use openmodel::primitives::{Point, Vector, Xform};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ArrowTransform {
    pub cylinder_transform: [f32; 16],  // 4x4 matrix for cylinder shaft
    pub cylinder_color: [f32; 3],       // Color for the shaft
    pub cylinder_thickness: f32,        // Thickness of the shaft
    pub cone_transform: [f32; 16],      // 4x4 matrix for cone head
    pub cone_color: [f32; 3],          // Color for the cone head
    pub cone_thickness: f32,            // Size of the cone head
    pub padding: [f32; 2],              // Padding for alignment
}

// Verify struct size for GPU alignment - 44 f32s = 176 bytes
// static_assertions::const_assert_eq!(std::mem::size_of::<ArrowTransform>(), 44 * 4);

impl ArrowTransform {
    pub fn from_arrow(arrow: &Arrow) -> Option<Self> {
        // Check if we have a transformation matrix in the JSON data
        let json_transform = arrow.data.transformation();
        let _is_identity = json_transform.iter().enumerate().all(|(i, &val)| {
            (i.is_multiple_of(5) && (val - 1.0).abs() < f32::EPSILON) || // Diagonal elements are 1.0
            (!i.is_multiple_of(5) && val.abs() < f32::EPSILON) // Non-diagonal elements are 0.0
        });

        // Calculate transforms with proper proportions
        let cylinder_xform = Self::calculate_cylinder_transform(arrow)?;
        let cone_xform = Self::calculate_cone_transform(arrow)?;
        let (cylinder_transform, cone_transform) = (cylinder_xform.m, cone_xform.m);
        
        // Get color and thickness from arrow data
        let color = arrow.data.get_color();
        let gpu_color = [color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0];
        let thickness = arrow.data.get_thickness();
        
        // Cone should be slightly larger than the shaft
        let cone_size = thickness * 1.5;
        
        let result = Self {
            cylinder_transform,
            cylinder_color: gpu_color,
            cylinder_thickness: thickness,
            cone_transform,
            cone_color: gpu_color,
            cone_thickness: cone_size,
            padding: [0.0, 0.0],
        };
        
        // Debug transform matrices
        
        Some(result)
    }
    
    /// Calculate transform for cylinder that ends at cone base
    fn calculate_cylinder_transform(arrow: &Arrow) -> Option<Xform> {
        let start = Point::new(arrow.x0, arrow.y0, arrow.z0);
        let end = Point::new(arrow.x1, arrow.y1, arrow.z1);
        
        // Direction vector from start to end
        let dir = Vector::new(end.x - start.x, end.y - start.y, end.z - start.z);
        let len = dir.length();
        let eps = 1e-9;
        if len < eps { return None; }
        
        let axis = dir.normalize();
        let z_axis = Vector::new(0.0, 0.0, 1.0);
        
        // Use fixed cone height based only on thickness, not arrow length
        let thickness = arrow.data.get_thickness();
        let cone_height = thickness * 3.0; // Fixed ratio to thickness
        let cylinder_length = len - cone_height;
        
        
        // Calculate cylinder end point (where cone base will be)
        let cylinder_end = Point::new(
            start.x + axis.x * cylinder_length,
            start.y + axis.y * cylinder_length,
            start.z + axis.z * cylinder_length
        );
        
        // Rotation aligning +Z to the arrow direction
        let mut dot = axis.dot(&z_axis);
        dot = dot.clamp(-1.0, 1.0);
        let rotation = if (dot - 1.0).abs() < eps {
            Xform::identity()
        } else if (dot + 1.0).abs() < eps {
            // +Z to -Z: 180° around any axis perpendicular to Z (choose X)
            Xform::rotation_x(std::f32::consts::PI)
        } else {
            let rot_axis = z_axis.cross(&axis).normalize();
            let angle = dot.acos();
            Xform::rotation(&rot_axis, angle)
        };
        
        // Position cylinder at midpoint between start and cylinder end
        let midpoint = Point::new(
            (start.x + cylinder_end.x) * 0.5,
            (start.y + cylinder_end.y) * 0.5,
            (start.z + cylinder_end.z) * 0.5,
        );
        let translation = Xform::translation(midpoint.x, midpoint.y, midpoint.z);
        
        // Scale cylinder to proper length
        let scale = Xform::scaling(1.0, 1.0, cylinder_length);
        
        // Compose T * R * S (scale → rotate → translate)
        Some(translation * rotation * scale)
    }
    
    /// Calculate transform for cone at arrow end point
    fn calculate_cone_transform(arrow: &Arrow) -> Option<Xform> {
        let start = Point::new(arrow.x0, arrow.y0, arrow.z0);
        let end = Point::new(arrow.x1, arrow.y1, arrow.z1);
        
        // Direction vector from start to end
        let dir = Vector::new(end.x - start.x, end.y - start.y, end.z - start.z);
        let len = dir.length();
        let eps = 1e-9;
        if len < eps { return None; }
        
        let axis = dir.normalize();
        let z_axis = Vector::new(0.0, 0.0, 1.0);
        
        // Use fixed cone height based only on thickness, not arrow length
        let thickness = arrow.data.get_thickness();
        let cone_height = thickness * 3.0; // Fixed ratio to thickness
        
        // Position cone base at the point where cylinder should end
        let cone_base_offset = len - cone_height;
        let cone_base_pos = Point::new(
            start.x + axis.x * cone_base_offset,
            start.y + axis.y * cone_base_offset,
            start.z + axis.z * cone_base_offset
        );
        
        // Rotation aligning +Z to the arrow direction
        let mut dot = axis.dot(&z_axis);
        dot = dot.clamp(-1.0, 1.0);
        let rotation = if (dot - 1.0).abs() < eps {
            Xform::identity()
        } else if (dot + 1.0).abs() < eps {
            // +Z to -Z: 180° around any axis perpendicular to Z (choose X)
            Xform::rotation_x(std::f32::consts::PI)
        } else {
            let rot_axis = z_axis.cross(&axis).normalize();
            let angle = dot.acos();
            Xform::rotation(&rot_axis, angle)
        };
        
        // Position cone at the base position
        let translation = Xform::translation(cone_base_pos.x, cone_base_pos.y, cone_base_pos.z);
        
        // Scale cone: XY should be 2x cylinder thickness, Z is the cone height
        let cone_radius = thickness * 2.0;
        let scale = Xform::scaling(cone_radius, cone_radius, cone_height);
        
        // Compose T * R * S (scale → rotate → translate)
        Some(translation * rotation * scale)
    }
}

/// Arrow pipeline that renders arrows with cylinder shafts and cone heads
pub struct ArrowPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub data_bind_group_layout: wgpu::BindGroupLayout,
}

impl ArrowPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        depth_format: wgpu::TextureFormat,
        msaa_sample_count: u32,
    ) -> Self {
        // Use the dedicated arrow cone shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader_arrow_cone.wgsl"));

        // Create data bind group layout for arrow transforms
        let data_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("arrow_data_bind_group_layout"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Arrow Render Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout, &data_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Arrow Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // Use the cone vertex shader
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE), // Change from alpha blending to replace
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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
        });

        Self {
            render_pipeline,
            data_bind_group_layout,
        }
    }

    pub fn update_data(
        &mut self,
        device: &wgpu::Device,
        arrow_transforms: Vec<ArrowTransform>,
    ) -> wgpu::BindGroup {
        
        let data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Arrow Data Buffer"),
            contents: bytemuck::cast_slice(&arrow_transforms),
            usage: wgpu::BufferUsages::STORAGE,
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.data_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: data_buffer.as_entire_binding(),
            }],
            label: Some("arrow_data_bind_group"),
        })
    }

    pub fn render_arrows(
        &self,
        render_pass: &mut wgpu::RenderPass,
        data_bind_group: &wgpu::BindGroup,
        camera_bind_group: &wgpu::BindGroup,
        num_arrows: u32,
    ) {
        if num_arrows == 0 { return; }
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, data_bind_group, &[]);
        // Draw each arrow individually to debug
        for i in 0..num_arrows {
            let start_vertex = i * 84;
            let end_vertex = start_vertex + 84;
            render_pass.draw(start_vertex..end_vertex, 0..1);
        }
    }
}

/// Convert a single arrow to an arrow transform
pub fn arrow_to_arrow_transform(arrow: &Arrow) -> Option<ArrowTransform> {
    ArrowTransform::from_arrow(arrow)
}

/// Convert a slice of arrows to a vector of arrow transforms
pub fn arrows_to_arrow_transforms(arrows: &[Arrow]) -> Vec<ArrowTransform> {
    arrows.iter()
        .filter_map(arrow_to_arrow_transform)
        .collect()
}

