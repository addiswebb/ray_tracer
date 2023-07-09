use std::{io::{BufReader, Cursor}, path::Path};

use super::context::{Mesh, Vertex};

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"));

pub async fn load_string(path: &Path) -> anyhow::Result<String> {
    assert!(
        path.exists(),
        "Text file does not exist: {}",
        path.display()
    );

    Ok(std::fs::read_to_string(path)?)
}

pub async fn load_binary(path: &Path) -> anyhow::Result<Vec<u8>> {
    assert!(
        path.exists(),
        "Binary file does not exist: {}",
        path.display()
    );

    Ok(std::fs::read(path)?)
}

pub async fn load_model(
    path: &Path,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    meshes: &mut Vec<Mesh>
) -> anyhow::Result<()>{
    let path = std::path::Path::new(FILE).join("assets").join(path);

    log::info!("Loading model: {}", path.display());
    if path.extension() == Some("obj".as_ref()) {
        load_model_obj(&path, vertices, indices, meshes).await
    } else if path.extension() == Some("gltf".as_ref()) {
        load_model_gltf(&path, vertices, indices, meshes).await
    } else if path.extension() == Some("glb".as_ref()) {
        load_model_glb(&path, vertices, indices, meshes).await
    } else {
        Err(anyhow::anyhow!("Unsupported model format"))
    }
}

pub async fn load_model_obj(
    path: &Path,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    meshes: &mut Vec<Mesh>
) -> anyhow::Result<()> {

    let obj_text = load_string(path).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);
    let (models, _obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&Path::new("assets").join(path.parent().unwrap()).join(&p)).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    for mut m in models{
        vertices.append(&mut (0..m.mesh.positions.len() / 3)
            .map(|i| Vertex{
                pos: [
                    m.mesh.positions[i * 3],
                    m.mesh.positions[i * 3 + 1],
                    m.mesh.positions[i * 3 + 2],
                ],
                _padding1: 0.0,
                normal: [
                    m.mesh.normals[i * 3],
                    m.mesh.normals[i * 3 + 1],
                    m.mesh.normals[i * 3 + 2],
                ],
                _padding2: 0.0,
            })
            .collect::<Vec<_>>());
            meshes.push(Mesh{
                length: m.mesh.indices.len() as u32,
                offset: indices.len() as u32 - 1,
                _padding2: [0.0;2],
                pos: [meshes.len() as f32 * 3.0, 0.0,0.0],
                _padding: 0.0,
                color: [0.2,0.2,1.0,1.0],
                emission_color: [0.0;4],
                emission_strength: 0.0,
                _padding3: [0.0;3],
            });
            indices.append(&mut m.mesh.indices);
    }
    Ok(())
}

pub async fn load_model_gltf(
    path: &Path,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    meshes: &mut Vec<Mesh>
) -> anyhow::Result<()> {
    let gltf_text = load_string(path).await?;
    let gltf_cursor = Cursor::new(gltf_text);
    let gltf_reader = BufReader::new(gltf_cursor);
    let gltf = gltf::Gltf::from_reader(gltf_reader)?;

    // Load buffers
    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        let bin = match buffer.source() {
            gltf::buffer::Source::Bin => {
                Vec::new()
            }
            gltf::buffer::Source::Uri(uri) => {
                let uri = path.with_file_name(uri);
                load_binary(&uri).await?
            }
        };

        buffer_data.push(bin);
    }

    log::debug!("Initizalized buffers");

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            log::info!("Node {} {}", node.index(), node.name().unwrap_or("Unnamed"));

            let mesh = node.mesh().expect("Failed to get mesh");
            let primitives = mesh.primitives();
            primitives.for_each(|primitive| {
                let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));

                log::info!("[START] Reading positions, normals");
                let (positions, normals) = (
                    reader.read_positions().unwrap(),
                    reader.read_normals().unwrap(),
                );
                log::info!("[END  ] Reading positions, normals");

                log::info!("[START] Reading indices");
                let i = reader.read_indices().map(|indices| indices.into_u32());
                let mut new_indices = match i{
                    Some(indices) => indices.collect::<Vec<_>>(),
                    None => (0..positions.len() as u32).collect(),
                };
                log::info!("[END  ] Reading indices");

                vertices.append(&mut positions
                    .zip(normals)
                    .map(|(pos, normal)| Vertex {
                        pos: [pos[0]* meshes.len() as f32,pos[1]* meshes.len() as f32,pos[2]* meshes.len() as f32],
                        _padding1: 0.0,
                        normal,
                        _padding2: 0.0,
                    })
                    .collect::<Vec<Vertex>>());

                meshes.push(Mesh{
                    length: new_indices.len() as u32,
                    offset: indices.len() as u32 - 1,
                    _padding2: [0.0;2],
                    pos: [meshes.len() as f32 * 3.0, 0.0,0.0],
                    _padding: 0.0,
                    color: [0.2,0.2,1.0,1.0],
                    emission_color: [0.0;4],
                    emission_strength: 0.0,
                    _padding3: [0.0;3],
                });
                indices.append(&mut new_indices);
            });
        }
    }

    Ok(())
}

pub async fn load_model_glb(
    path: &Path,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    meshes: &mut Vec<Mesh>
) -> anyhow::Result<()> {
    let gltf_text = load_binary(path).await?;
    let gltf_cursor = Cursor::new(gltf_text);
    let gltf_reader = BufReader::new(gltf_cursor);
    let gltf = gltf::Gltf::from_reader(gltf_reader)?;

    // Load buffers
    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        let bin = match buffer.source() {
            gltf::buffer::Source::Bin => {
                if let Some(blob) = gltf.blob.clone() {
                    blob
                } else {
                    log::error!("Missing blob");
                    return Err(anyhow::anyhow!("Missing blob"));
                }
            }
            gltf::buffer::Source::Uri(uri) => {
                let uri = path.with_file_name(uri);
                load_binary(&uri).await?
            }
        };

        buffer_data.push(bin);
    }

    for mesh in gltf.meshes() {
        log::info!(
            r#"Mesh#{} "{}""#,
            mesh.index(),
            mesh.name().unwrap_or("Unnamed")
        );

        let primitives = mesh.primitives();
        primitives.for_each(|primitive| {
            let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));

            log::info!("[START] Reading positions, normals");
            let (positions, normals) = (
                reader.read_positions().unwrap(),
                reader.read_normals().unwrap(),
            );
            log::info!("[END  ] Reading positions, normals");

            log::info!("[START] Reading indices");
            let i = reader.read_indices().map(|indices| indices.into_u32());
            let mut new_indices = match i{
                Some(indices) => indices.collect::<Vec<_>>(),
                None => (0..positions.len() as u32).collect(),
            };
            log::info!("[END  ] Reading indices");

            vertices.append(&mut positions
                .zip(normals)
                .map(|(pos, normal)| Vertex {
                    pos,
                    _padding1: 0.0,
                    normal,
                    _padding2: 0.0,
                })
                .collect::<Vec<Vertex>>());

            meshes.push(Mesh{
                length: new_indices.len() as u32,
                offset: indices.len() as u32 - 1,
                _padding2: [0.0;2],
                pos: [meshes.len() as f32 * 3.0, 0.0,0.0],
                _padding: 0.0,
                color: [0.2,0.2,1.0,1.0],
                emission_color: [0.0;4],
                emission_strength: 0.0,
                _padding3: [0.0;3],
            });

            indices.append(&mut new_indices);
        });
    }
    Ok(())
}