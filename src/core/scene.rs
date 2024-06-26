use std::{path::Path};

use glam::{Vec3, Vec4};
use rand::Rng;
use wgpu::util::DeviceExt;

use crate::core::resource::load_model;

use super::camera::Camera;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Sphere{
    position: [f32;3], 
    radius: f32,
    color: [f32;4],
    emission_color: [f32;4],
    emission_strength: f32,
    smoothness: f32,
    _padding: [f32;2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Mesh{
    pub first: u32,
    pub triangles: u32,
    pub offset: u32,
    pub _padding: f32,
    pub pos: [f32;3],
    pub _padding2: f32,
    pub color: [f32;4],
    pub emission_color: [f32;4],
    pub emission_strength: f32,
    pub specular: f32,
    pub _padding3: [f32;2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Material{
    color: [f32;4],
    emission_color: [f32;4],
    emission_strength: f32,
}

impl Sphere{
    pub fn new(position: Vec3, radius: f32, color: Vec4, emission_color: Vec4, emission_strength: f32, specular: f32) -> Self{
        Self { 
            position: position.to_array(),
            radius,
            color: color.to_array(),
            emission_color: emission_color.to_array(),
            emission_strength,
            _padding: [0.0;2],
            smoothness: if specular < 1.0 {specular} else{1.0},
        }
    }
}

impl Mesh{
    pub fn new(pos: Vec3, first: u32, triangles: u32,offset: u32,color: Vec4, emission_color: Vec4, emission_strength: f32,specular: f32)->Self{
        Self{
            first,
            triangles,
            offset,
            _padding2: 0.0,
            pos: pos.to_array(),
            _padding: 0.0,
            color: color.to_array(),
            emission_color: emission_color.to_array(),
            emission_strength,
            specular: if specular < 1.0 {specular} else{1.0},
            _padding3: [0.0;2],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex{
    pub pos: [f32;3],
    pub _padding1: f32,
    pub normal: [f32;3],
    pub _padding2: f32,
}

impl Vertex{
    pub fn new(pos: Vec3, normal: Vec3) -> Self{
        Self { pos: pos.to_array(), _padding1: 0.0, normal: normal.to_array(), _padding2: 0.0 }
    }
}

pub struct Scene{
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>, 
    pub meshes: Vec<Mesh>,
}

impl Scene{
    pub fn new(device: &wgpu::Device,config: &wgpu::SurfaceConfiguration)->Self{
        let camera = Camera::new(&device,
            Vec3::new(-2.764473, 5.8210998, 3.839141),
            Vec3::new(-2.0999293, 5.1703076, 3.4719195),
            Vec3::new(0.0,1.0,0.0),45.0,
            config.width as f32/config.height as f32,0.1,100.0,
            1.0,
            2.0,
        );
        Self{
            camera,
            spheres: vec![],
            vertices: vec![],
            indices: vec![],
            meshes: vec![],
        }
    }

    pub fn random_balls(device: &wgpu::Device,config: &wgpu::SurfaceConfiguration)->Self{
        let camera = Camera::new(&device,
            Vec3::new(10.5,2.0,3.0),
            Vec3::new(0.0,0.0,0.0),
            Vec3::new(0.0,1.0,0.0),45.0,
            config.width as f32/config.height as f32,0.1,100.0,
            0.1,
            10.0,
        );
        let mut spheres: Vec<Sphere> = vec![
            Sphere::new(
                Vec3::new(0.0,-1000.0,0.0),1000.0,Vec4::new(0.5,0.5,0.5,1.0),Vec4::ZERO,0.0,0.0
            )
        ];

        for a in -11..11{
            for b in -11..11{
                let mut rng = rand::thread_rng();

                let mat = rng.gen::<f32>();

                let center = Vec3::new(a as f32 + 0.9 * rng.gen::<f32>(),0.2,b as f32 + 0.9 * rng.gen::<f32>());
                if (center - Vec3::new(4.0,0.2,0.0)).length() > 0.9{
                    if mat < 0.8{
                        let albedo = Vec4::new(rng.gen::<f32>(),rng.gen::<f32>(),rng.gen::<f32>(),1.0);
                        spheres.push(Sphere::new(
                            center,0.2,albedo,Vec4::ZERO,0.0,0.0
                        ));
                    }else if mat < 0.95{
                        let albedo = Vec4::new(rng.gen_range(0.5..1.0),rng.gen_range(0.5..1.0),rng.gen_range(0.5..1.0),1.0);
                        let fuzz = rng.gen_range(0.0..0.5);
                        spheres.push(Sphere::new(
                            center,0.2,albedo,Vec4::ZERO,0.0,fuzz
                        ));
                    }else{
                        spheres.push(Sphere::new(
                            center,0.2,Vec4::ONE,Vec4::ZERO,0.0,-1.0,
                        ));
                    }
                }
            }
        }

        spheres.push(Sphere::new(
            Vec3::new(0.0,1.0,0.0),1.0,Vec4::ONE,Vec4::ZERO,0.0,-1.0
        ));

        spheres.push(Sphere::new(
            Vec3::new(-4.0,1.0,0.0),1.0,Vec4::new(0.4,0.2,0.1,1.0),Vec4::ZERO,0.0,0.0
        ));

        spheres.push(Sphere::new(
            Vec3::new(4.0,1.0,0.0),1.0,Vec4::new(0.7,0.6,0.5,1.0),Vec4::ZERO,0.0,0.9
        ));

        let vertices = vec![
            Vertex::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0,-3.0,-1.0)),
        ];
        let indices = vec![
            1u32,
        ];
        let meshes = vec![
            Mesh::new(
                Vec3::new(0.0,0.0,0.0),
                0, 0, 0,
                Vec4::new(0.0,0.6,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
        ];
        Self{
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }
    pub fn room(device: &wgpu::Device,config: &wgpu::SurfaceConfiguration)->Self{
        let camera = Camera::new(&device,
            Vec3::new(-7.0,0.0,0.0),
            Vec3::new(1.0,0.0,0.0),
            Vec3::new(0.0,1.0,0.0),45.0,
            config.width as f32/config.height as f32,0.1,100.0,
            0.0,
            0.1,
        );

        let spheres = vec![
            Sphere::new(
                Vec3::new(4.0,0.0,1.7),1.2, 
                Vec4::new(1.0,1.0,1.0,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                1.0
            ),
            Sphere::new(
                Vec3::new(4.0,0.0,-1.7),1.2, 
                Vec4::new(1.0,1.0,1.0,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                0.5
            ),
        ];

        #[allow(unused_mut)]
        let mut vertices = vec![
            Vertex::new(Vec3::new(3.0,-3.0,-3.0), Vec3::new(2.0,-3.0,-3.0)),
            Vertex::new(Vec3::new(3.0,-3.0, 3.0), Vec3::new(4.0,-3.0, 0.0)),
            Vertex::new(Vec3::new(-3.0,-3.0,3.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(-3.0,-3.0,-3.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(3.0,3.0,-3.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(3.0,3.0,3.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(-3.0,3.0,3.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(-3.0,3.0,-3.0), Vec3::new(3.0,-4.0, 2.0)),

            Vertex::new(Vec3::new(1.0,1.0,-1.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(1.0,1.0,1.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(-1.0,1.0,1.0), Vec3::new(3.0,-4.0, 2.0)),
            Vertex::new(Vec3::new(-1.0,1.0,-1.0), Vec3::new(3.0,-4.0, 2.0)),
        ];
        #[allow(unused_mut)]
        let mut indices = vec![
            3u32, 2u32, 1u32,
            3u32, 1u32, 0u32,
            7u32, 0u32, 4u32,
            7u32, 3u32, 0u32,
            7u32, 6u32, 2u32,
            7u32, 2u32, 3u32,
            2u32, 6u32, 5u32,
            2u32, 5u32, 1u32,
            1u32, 5u32, 4u32,
            1u32, 4u32, 0u32,
            5u32, 6u32, 7u32,
            5u32, 7u32, 4u32,

            9u32, 10u32, 11u32,
            9u32, 11u32, 8u32,
        ];
        #[allow(unused_mut)]
        let mut meshes = vec![
            Mesh::new(
                Vec3::new(3.0,0.0,0.0),
                0, 2, 0,
                Vec4::new(1.0,0.0,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
            Mesh::new(
                Vec3::new(3.0,0.0,0.0),
                6, 2, 0,
                Vec4::new(0.0,1.0,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
            Mesh::new(
                Vec3::new(3.0,0.0,0.0),
                12, 2, 0,
                Vec4::new(0.0,0.0,1.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
            Mesh::new(
                Vec3::new(3.0,0.0,0.0),
                18, 2, 0,
                Vec4::new(0.5,0.5,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
            Mesh::new(
                Vec3::new(3.0,0.0,0.0),
                24, 2, 0,
                Vec4::new(0.0,0.5,0.5,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
            Mesh::new(
                Vec3::new(3.0,0.0,0.0),
                30, 2, 0,
                Vec4::new(1.0,1.0,1.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
            Mesh::new(
                Vec3::new(3.0,1.9,0.0),
                36, 2, 0,
                Vec4::new(1.0,1.0,1.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 10.5, 0.0,
            ),
        ];

        Self{
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }
    pub fn metal(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration)->Self{
        let lookfrom = Vec3::new(0.0,0.0,3.0);
        let lookat = Vec3::new(0.0,0.0,-1.0);
        // let lookfrom= Vec3::new(3.0,3.0,2.0);
        // let lookat = Vec3::new(0.0,0.0,-1.0);
        let length = (lookfrom - lookat).length();

        let camera = Camera::new(&device,
            lookfrom,
            lookat,
            Vec3::new(0.0,1.0,0.0),45.0,
            config.width as f32/config.height as f32,0.1,100.0,
            // 2.0,
            // length,
            0.0,
            0.1,
        );
        println!("{:?}",camera);
        let spheres = vec![
            //floor
            Sphere::new(
                Vec3::new(0.0, -100.5,-1.0),100.0, 
                Vec4::new(0.8,0.8,0.0,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                0.0,
            ),
            Sphere::new(
                Vec3::new(0.0, 0.0, -1.0),0.5, 
                Vec4::new(0.7,0.3,0.3,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                0.0,
            ),
            Sphere::new(
                Vec3::new(-1.0, 0.0, -1.0),0.5, 
                Vec4::new(0.8,0.8,0.8,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                -1.0,
            ),
            Sphere::new(
                Vec3::new(1.0, 0.0, -1.0),0.5, 
                Vec4::new(0.8,0.6,0.2,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                0.15,
            ),
        ];
        //TODO allow for no meshes or sphers in a scene
        let vertices = vec![
            Vertex::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0,-3.0,-1.0)),
        ];
        let indices = vec![
            1u32,
        ];
        let meshes = vec![
            Mesh::new(
                Vec3::new(0.0,0.0,0.0),
                0, 0, 0,
                Vec4::new(0.0,0.6,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
        ];
        Self{
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }
    pub fn balls(device: &wgpu::Device,config: &wgpu::SurfaceConfiguration)->Self{
        let camera = Camera::new(&device,
            Vec3::new(3.089,1.53,-3.0),
            Vec3::new(-2.0,-1.0,2.0),
            Vec3::new(0.0,1.0,0.0),45.0,
            config.width as f32/config.height as f32,0.1,100.0,
            0.0,
            0.1,
        );
/*         let camera = Camera::new(&device,Vec3::new(-5.0,-10.0,0.0),Vec3::new(-2.0,-3.0,0.0),Vec3::new(0.0,1.0,0.0),45.0,config.width as f32/config.height as f32,0.1,100.0); */
/*         let camera = Camera::new(&device,Vec3::new(-2.7,1.3,-8.0),Vec3::new(-2.6,1.0,-7.0),Vec3::new(0.0,1.0,0.0),28.0,config.width as f32/config.height as f32,0.1,100.0); */
        println!("{} {}",camera.pitch, camera.yaw);

        let spheres = vec![
            Sphere::new(
                Vec3::new(-3.64,-0.42,0.8028),0.75, 
                Vec4::new(1.0,1.0,1.0,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                0.7
            ),
            Sphere::new(
                Vec3::new(-2.54,-0.72,0.5),0.6, 
                Vec4::new(1.0,0.0,0.0,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
                0.5
            ),
            Sphere::new(
                Vec3::new(-1.27,-0.72,1.0),0.5, 
                Vec4::new(0.0,1.0,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0,
                0.2
            ),
            Sphere::new(
                Vec3::new(-0.5,-0.9,1.55),0.35,
                Vec4::new(0.0,0.0,1.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0,
                0.0
            ),
            /*  floor*/
            Sphere::new(
                Vec3::new(-3.46,-15.88,2.76),15.0,
                Vec4::new(0.5,0.0,0.8,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0,
                0.0
            ),
            /*  Light Object       */
            Sphere::new(
                Vec3::new(-7.44,-0.72,20.0),15.0,
                Vec4::new(0.1,0.1,0.1,0.0),
                Vec4::new(1.0,1.0,1.0,1.0), 2.0,
                0.0
            )
        ];
        let vertices = vec![
            Vertex::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0,-3.0,-1.0)),
        ];
        let indices = vec![
            1u32,
        ];
        let meshes = vec![
            Mesh::new(
                Vec3::new(0.0,0.0,0.0),
                0, 0, 0,
                Vec4::new(0.0,0.6,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
            ),
        ];
        // #[allow(unused_mut)]
        // let mut vertices = vec![
        //     Vertex::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0,-3.0,-1.0)),
        //     Vertex::new(Vec3::new(0.0, 0.15, 0.0), Vec3::new(4.0,-3.0, 0.0)),
        //     Vertex::new(Vec3::new(1.0, 0.3, 0.0), Vec3::new(3.0,-4.0, 2.0)),
        // ];
        // #[allow(unused_mut)]
        // let mut indices = vec![
        //     2u32,1u32,0u32,
        // ];
        // #[allow(unused_mut)]
        // let mut meshes = vec![
        //     Mesh::new(
        //         Vec3::new(0.0,0.0,0.0),
        //         0, 1, 0,
        //         Vec4::new(0.0,0.6,0.0,1.0),
        //         Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
        //     ),
        // ];

        //load_model(Path::new("cube2.obj"),&mut vertices, &mut indices, &mut meshes).await.unwrap();


        Self{
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }

    pub fn sphere_buffer(&self, device: &wgpu::Device)->wgpu::Buffer{
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Sphere Buffer"),
            contents: bytemuck::cast_slice(&self.spheres),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        })
    }
    pub fn vertex_buffer(&self, device: &wgpu::Device)->wgpu::Buffer{
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        })
    }

    pub fn index_buffer(&self, device: &wgpu::Device)->wgpu::Buffer{
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        })
    }

    pub fn mesh_buffer(&self, device: &wgpu::Device)->wgpu::Buffer{
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Mesh Buffer"),
            contents: bytemuck::cast_slice(&self.meshes),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        })
    }
}