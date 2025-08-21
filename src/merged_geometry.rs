use crate::vertex::Vertex;
use crate::instance::{DrawBatch, BatchKind, Instance};
use openmodel::geometry::Mesh;
use openmodel::AllGeometryData;

/// Helper function to merge all pipe and sphere instances into single buffers
/// This eliminates instanced rendering overhead by pre-transforming geometry
pub fn create_merged_geometry(all_geom: &AllGeometryData) -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut batches: Vec<DrawBatch> = Vec::new();

    // Helper function to append mesh triangles
    fn append_mesh_as_triangles(
        mesh: &mut Mesh,
        default_color: [f32; 3],
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u16>,
    ) {
        // Collect face keys first to avoid holding an immutable borrow while mutably triangulating
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
                            [c[0] as f32, c[1] as f32, c[2] as f32]
                        } else { default_color };

                        // Resolve vertex normal via OpenModel (authored -> computed -> face -> +Z)
                        let n = mesh.vertex_normal_resolved(vk, Some(face_key));
                        let normal = [n.x as f32, n.y as f32, n.z as f32];

                        if vertices.len() >= u16::MAX as usize { break; }
                        vertices.push(Vertex { position: [pos.x as f32, pos.y as f32, pos.z as f32], color, normal });
                        indices.push((vertices.len() - 1) as u16);
                    }
                }
            }
        }
    }

    // MERGED PIPES: Transform and merge all pipe instances into single buffer
    if let Some(pipe_idx) = all_geom.pipe_mesh_index {
        if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == pipe_idx) {
            if !mi.transforms.is_empty() {
                let unit_pipe = Mesh::create_unit_pipe_low_res();
                let first_index = indices.len() as u32;
                
                // Generate transformed geometry for each pipe instance
                for xf in &mi.transforms {
                    let mut pipe_copy = unit_pipe.clone();
                    // Apply transform to each vertex
                    for (_, vertex_data) in pipe_copy.vertex.iter_mut() {
                        let pos = vertex_data.position();
                        let transformed = xf.transform_point(&pos);
                        vertex_data.set_position(transformed);
                    }
                    append_mesh_as_triangles(&mut pipe_copy, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
                }
                
                let index_count = (indices.len() as u32) - first_index;
                if index_count > 0 {
                    batches.push(DrawBatch {
                        first_index,
                        index_count,
                        base_vertex: 0,
                        instances: vec![Instance::identity()], // Single identity instance
                        kind: BatchKind::Surface, // Treat as regular surface geometry
                    });
                    log::info!(
                        "Created merged pipe batch: {} pipes, index_count={}",
                        mi.transforms.len(),
                        index_count
                    );
                }
            }
        }
    }

    // MERGED SPHERES: Transform and merge all sphere instances into single buffer
    if let Some(sphere_idx) = all_geom.sphere_mesh_index {
        if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == sphere_idx) {
            if !mi.transforms.is_empty() {
                let unit_sphere = Mesh::create_unit_sphere_low_res();
                let first_index = indices.len() as u32;
                
                // Generate transformed geometry for each sphere instance
                for xf in &mi.transforms {
                    let mut sphere_copy = unit_sphere.clone();
                    // Apply transform to each vertex
                    for (_, vertex_data) in sphere_copy.vertex.iter_mut() {
                        let pos = vertex_data.position();
                        let transformed = xf.transform_point(&pos);
                        vertex_data.set_position(transformed);
                    }
                    append_mesh_as_triangles(&mut sphere_copy, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
                }
                
                let index_count = (indices.len() as u32) - first_index;
                if index_count > 0 {
                    batches.push(DrawBatch {
                        first_index,
                        index_count,
                        base_vertex: 0,
                        instances: vec![Instance::identity()], // Single identity instance
                        kind: BatchKind::Surface, // Treat as regular surface geometry
                    });
                    log::info!(
                        "Created merged sphere batch: {} spheres, index_count={}",
                        mi.transforms.len(),
                        index_count
                    );
                }
            }
        }
    }

    (vertices, indices, batches)
}
