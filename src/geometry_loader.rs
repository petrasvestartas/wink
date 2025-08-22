#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::{JsCast, UnwrapThrowExt},
    wasm_bindgen_futures::JsFuture,
    web_sys::{Request, RequestInit, RequestCache},
    std::cell::{Cell, RefCell},
};

use crate::vertex::Vertex;
use crate::instance::{DrawBatch, BatchKind, Instance};
use crate::shader_pointcloud_pipeline::PointCloudInstance;
use crate::shader_geometry_pipeline::{PipeTransform, SphereTransform};
use crate::error_handling::ErrorHandler;
use openmodel::{AllGeometryData, geometry::Mesh};

/// Geometry loading and processing utilities
pub struct GeometryLoader;

#[cfg(target_arch = "wasm32")]
thread_local! {
    pub static PENDING_GEOMETRY: RefCell<Option<(Vec<crate::vertex::Vertex>, Vec<u16>, Vec<crate::instance::DrawBatch>, Vec<crate::shader_pointcloud_pipeline::PointCloudInstance>, Vec<crate::shader_geometry_pipeline::PipeTransform>, Vec<crate::shader_geometry_pipeline::SphereTransform>, [[f32; 4]; 4])>> = RefCell::new(None);
    pub static LOCAL_HASH: RefCell<Option<u64>> = RefCell::new(None);
    pub static LOCAL_FETCHING: Cell<bool> = Cell::new(false);
}

#[cfg(target_arch = "wasm32")]
impl GeometryLoader {
    /// Set up WASM panic hook and run the main application
    pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
        console_error_panic_hook::set_once();
        crate::run().unwrap_throw();
        Ok(())
    }

    /// Check if geometry is currently being fetched
    pub fn is_fetching() -> bool {
        LOCAL_FETCHING.with(|f| f.get())
    }

    /// Set fetching state
    pub fn set_fetching(fetching: bool) {
        LOCAL_FETCHING.with(|f| f.set(fetching));
    }

    /// Get pending geometry data
    pub fn take_pending_geometry() -> Option<(Vec<crate::vertex::Vertex>, Vec<u16>, Vec<crate::instance::DrawBatch>, Vec<crate::shader_pointcloud_pipeline::PointCloudInstance>, Vec<crate::shader_geometry_pipeline::PipeTransform>, Vec<crate::shader_geometry_pipeline::SphereTransform>, [[f32; 4]; 4])> {
        PENDING_GEOMETRY.with(|pg| pg.borrow_mut().take())
    }

    /// Set pending geometry data
    pub fn set_pending_geometry(data: (Vec<crate::vertex::Vertex>, Vec<u16>, Vec<crate::instance::DrawBatch>, Vec<crate::shader_pointcloud_pipeline::PointCloudInstance>)) {
        let identity_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        PENDING_GEOMETRY.with(|pg| *pg.borrow_mut() = Some((data.0, data.1, data.2, data.3, Vec::new(), Vec::new(), identity_matrix)));
    }

    /// Set pending geometry data with GPU data
    pub fn set_pending_geometry_with_gpu_data(data: (Vec<crate::vertex::Vertex>, Vec<u16>, Vec<crate::instance::DrawBatch>, Vec<crate::shader_pointcloud_pipeline::PointCloudInstance>, Vec<crate::shader_geometry_pipeline::PipeTransform>, Vec<crate::shader_geometry_pipeline::SphereTransform>, [[f32; 4]; 4])) {
        PENDING_GEOMETRY.with(|pg| *pg.borrow_mut() = Some(data));
    }

    /// Get local hash for change detection
    pub fn get_local_hash() -> Option<u64> {
        LOCAL_HASH.with(|lh| *lh.borrow())
    }

    /// Set local hash for change detection
    pub fn set_local_hash(hash: u64) {
        LOCAL_HASH.with(|lh| *lh.borrow_mut() = Some(hash));
    }

    /// HTTP path for local geometry JSON
    pub const LOCAL_GEOMETRY_HTTP_PATH: &'static str = "/geometry/all_geometry.json";

    /// Fetch text content from URL
    pub async fn fetch_text(url: &str) -> Option<String> {
        // Add timestamp to URL to bust cache
        let timestamp = web_sys::window()?.performance()?.now() as u64;
        let cache_busted_url = format!("{}?t={}", url, timestamp);
        
        web_sys::console::log_1(&format!("🌐 Fetching URL: {}", cache_busted_url).into());
        
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_cache(RequestCache::NoCache);

        let request = Request::new_with_str_and_init(&cache_busted_url, &opts).ok()?;
        let window = web_sys::window()?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.ok()?;
        let resp: web_sys::Response = resp_value.dyn_into().ok()?;
        
        if !resp.ok() {
            return None;
        }

        let text_promise = resp.text().ok()?;
        let text_value = JsFuture::from(text_promise).await.ok()?;
        text_value.as_string()
    }

    /// FNV-1a 64-bit hash function
    pub fn fnv1a64(data: &[u8]) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;
        
        let mut hash = FNV_OFFSET_BASIS;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }
}

// Helper function to convert openmodel Xform to Instance
fn xform_to_instance(xf: &openmodel::primitives::Xform) -> Instance {
    Instance::from_xform(xf)
}

// Helper function to append mesh triangles
fn append_mesh_as_triangles(
    mesh: &mut Mesh,
    default_color: [f32; 3],
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
) {
    let face_keys: Vec<usize> = mesh.get_face_data().map(|(fk, _)| fk).collect();
    for face_key in face_keys {
        let triangles: Vec<[usize; 3]> = if let Some(tri_ref) = mesh.triangulate_face(face_key) {
            tri_ref.clone()
        } else {
            Vec::new()
        };

        for tri in triangles {
            for &vk in &tri {
                if let Some(pos) = mesh.vertex_position(vk) {
                    let use_default = if let Some(vd) = mesh.vertex.get(&vk) {
                        !(vd.attributes.contains_key("r") && vd.attributes.contains_key("g") && vd.attributes.contains_key("b"))
                    } else { true };
                    let color = if use_default {
                        default_color
                    } else if let Some(vd) = mesh.vertex.get(&vk) {
                        let c = vd.color();
                        [c[0], c[1], c[2]]
                    } else { default_color };

                    let n = mesh.vertex_normal_resolved(vk, Some(face_key));
                    let normal = [n.x, n.y, n.z];

                    if vertices.len() >= u16::MAX as usize { break; }
                    vertices.push(Vertex { position: [pos.x, pos.y, pos.z], color, normal });
                    indices.push((vertices.len() - 1) as u16);
                }
            }
        }
    }
}

impl GeometryLoader {
    /// Load geometry data using error handler
    pub async fn load_geometry_data() -> AllGeometryData {
        ErrorHandler::load_geometry_with_fallback().await
    }

    /// Extract pointcloud instances from geometry data
    pub fn extract_pointclouds(all_geom: &AllGeometryData) -> (Vec<PointCloudInstance>, [[f32; 4]; 4]) {
        let mut instances = Vec::new();
        let mut transform_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        
        for pc in &all_geom.point_clouds {
            // Extract transformation matrix from the first point cloud
            if instances.is_empty() {
                // Convert from column-major array to row-major 4x4 matrix
                let m = &pc.xform.m;
                transform_matrix = [
                    [m[0], m[4], m[8], m[12]],
                    [m[1], m[5], m[9], m[13]],
                    [m[2], m[6], m[10], m[14]],
                    [m[3], m[7], m[11], m[15]],
                ];
            }
            
            for (i, point) in pc.points.iter().enumerate() {
                let color = if i < pc.colors.len() {
                    let c = &pc.colors[i];
                    [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0]
                } else {
                    [1.0, 1.0, 1.0]
                };
                instances.push(PointCloudInstance {
                    position: [point.x, point.y, point.z],
                    size: 0.1,
                    color,
                });
            }
        }
        (instances, transform_matrix)
    }

    /// Extract GPU transforms from geometry data
    pub fn extract_gpu_transforms(all_geom: &AllGeometryData) -> (Vec<PipeTransform>, Vec<SphereTransform>) {
        let mut pipes = Vec::new();
        let mut spheres = Vec::new();
        
        // Extract pipe transforms
        if let Some(pipe_idx) = all_geom.pipe_mesh_index {
            if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == pipe_idx) {
                for xf in &mi.transforms {
                    pipes.push(PipeTransform::from(xf));
                }
            }
        }
        
        // Extract sphere transforms
        if let Some(sphere_idx) = all_geom.sphere_mesh_index {
            if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == sphere_idx) {
                for xf in &mi.transforms {
                    spheres.push(SphereTransform::from(xf));
                }
            }
        }
        
        (pipes, spheres)
    }

    /// Build mesh geometry (vertices, indices, batches)
    pub fn build_mesh_geometry(all_geom: &mut AllGeometryData) -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut batches = Vec::new();
        
        // Track mesh to batch mapping
        let mut mesh_to_batch: Vec<Option<usize>> = vec![None; all_geom.meshes.len()];
        let pipe_idx = all_geom.pipe_mesh_index;
        let sphere_idx = all_geom.sphere_mesh_index;
        
        // Process regular meshes (skip procedural pipe/sphere)
        for (i, m) in all_geom.meshes.iter_mut().enumerate() {
            if Some(i) == pipe_idx || Some(i) == sphere_idx { continue; }
            
            let first_index = indices.len() as u32;
            append_mesh_as_triangles(m, [0.8, 0.8, 0.8], &mut vertices, &mut indices);
            let index_count = (indices.len() as u32) - first_index;
            
            if index_count > 0 {
                batches.push(DrawBatch {
                    first_index,
                    index_count,
                    base_vertex: 0,
                    instances: vec![],
                    kind: BatchKind::Surface,
                });
                mesh_to_batch[i] = Some(batches.len() - 1);
            }
        }
        
        // Add pipe batch
        if let Some(pipe_idx) = pipe_idx {
            if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == pipe_idx) {
                let pipe_instances: Vec<Instance> = mi.transforms.iter().map(|xf| xform_to_instance(xf)).collect();
                if !pipe_instances.is_empty() {
                    let mut unit_pipe = Mesh::create_unit_pipe_high_res();
                    let first_index = indices.len() as u32;
                    append_mesh_as_triangles(&mut unit_pipe, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
                    let index_count = (indices.len() as u32) - first_index;
                    if index_count > 0 {
                        batches.push(DrawBatch {
                            first_index,
                            index_count,
                            base_vertex: 0,
                            instances: pipe_instances,
                            kind: BatchKind::Pipe,
                        });
                    }
                }
            }
        }
        
        // Add sphere batch
        if let Some(sphere_idx) = sphere_idx {
            if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == sphere_idx) {
                let sphere_instances: Vec<Instance> = mi.transforms.iter().map(|xf| xform_to_instance(xf)).collect();
                if !sphere_instances.is_empty() {
                    let mut unit_sphere = Mesh::create_unit_sphere_high_res();
                    let first_index = indices.len() as u32;
                    append_mesh_as_triangles(&mut unit_sphere, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
                    let index_count = (indices.len() as u32) - first_index;
                    if index_count > 0 {
                        batches.push(DrawBatch {
                            first_index,
                            index_count,
                            base_vertex: 0,
                            instances: sphere_instances,
                            kind: BatchKind::Sphere,
                        });
                    }
                }
            }
        }
        
        // Populate mesh instances into batches
        for mi in &all_geom.mesh_instances {
            if let Some(Some(bi)) = mesh_to_batch.get(mi.mesh_index) {
                let insts = mi.transforms.iter().map(|xf| xform_to_instance(xf)).collect();
                batches[*bi].instances = insts;
            }
        }
        
        (vertices, indices, batches)
    }

    /// Main geometry loading function - clean and focused
    pub async fn get_geometry() -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>, Vec<PointCloudInstance>, Vec<PipeTransform>, Vec<SphereTransform>, [[f32; 4]; 4]) {
        let mut all_geom = Self::load_geometry_data().await;
        let (vertices, indices, mut batches) = Self::build_mesh_geometry(&mut all_geom);
        let (pointcloud_instances, transform_matrix) = Self::extract_pointclouds(&all_geom);
        let (pipes, spheres) = Self::extract_gpu_transforms(&all_geom);
        
        // Add pointcloud batch if needed
        if !pointcloud_instances.is_empty() {
            batches.push(DrawBatch {
                first_index: 0,
                index_count: 0,
                base_vertex: 0,
                instances: Vec::new(),
                kind: BatchKind::PointCloud,
            });
        }
        
        (vertices, indices, batches, pointcloud_instances, pipes, spheres, transform_matrix)
    }
}
