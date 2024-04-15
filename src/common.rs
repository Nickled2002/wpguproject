#![allow(dead_code)]
use std:: {iter, mem };
//use std::f32::consts::PI;
use cgmath::Matrix4;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    window::Window,
    event_loop::{ControlFlow, EventLoop},
};
use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::VertexBufferLayout;
use std::time::{Instant, Duration};
use std::collections::VecDeque;



#[path="transforms.rs"]
mod transforms;
#[path="surface_data.rs"]
mod surface;

const X_CHUNKS_COUNT: u32 = 5;
const Z_CHUNKS_COUNT: u32 = 5;



#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CamPos{
    pub x:f32,
    pub y:f32,
    pub z:f32
}
pub struct IRenderPipeline<'a> {
    pub shader: Option<&'a wgpu::ShaderModule>,
    pub vs_shader: Option<&'a wgpu::ShaderModule>,
    pub fs_shader: Option<&'a wgpu::ShaderModule>,
    pub vertex_buffer_layout: &'a [VertexBufferLayout<'a>],
    pub pipeline_layout: Option<&'a wgpu::PipelineLayout>,
    pub topology: wgpu::PrimitiveTopology,
    pub strip_index_format: Option<wgpu::IndexFormat>,
    pub cull_mode: Option<wgpu::Face>,
    pub is_depth_stencil: bool,
    pub vs_entry: String,
    pub fs_entry: String,
}

pub struct IWgpuInit {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub sample_count: u32,
}

#[derive(Debug)]
pub struct FpsCounter {
    last_second_frames: VecDeque<Instant>,
    last_print_time: Instant,
}

struct State {
    init: IWgpuInit,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: Vec<wgpu::Buffer>,
    index_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    view_mat: Matrix4<f32>,
    project_mat: Matrix4<f32>,
    depth_texture_view: wgpu::TextureView,
    msaa_texture_view: wgpu::TextureView,
    index_length: u32,
    camera: CamPos,
    camlook: CamPos,
    translations: Vec<[f32; 2]>,
    terrain: surface::ITerrain,
    update_buffers: bool,
    update_buffers_view: bool,
    fps_counter: FpsCounter,
}

impl Default for IRenderPipeline<'_> {
    fn default() -> Self {
        Self {
            shader: None,
            vs_shader: None,
            fs_shader: None,
            vertex_buffer_layout: &[],
            pipeline_layout: None,
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            cull_mode: None,
            is_depth_stencil: true,
            vs_entry: String::from("vs_main"),
            fs_entry: String::from("fs_main"),
        }
    }
}

impl IRenderPipeline<'_> {
    pub fn new(&mut self, init: &IWgpuInit) -> wgpu::RenderPipeline {
        if self.shader.is_some() {
            self.vs_shader = self.shader;
            self.fs_shader = self.shader;
        }

        let mut depth_stencil:Option<wgpu::DepthStencilState> = None;
        if self.is_depth_stencil {
            depth_stencil = Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            });
        }

        init.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&self.pipeline_layout.unwrap()),
            vertex: wgpu::VertexState {
                module: &self.vs_shader.as_ref().unwrap(),
                entry_point: &self.vs_entry,
                buffers: &self.vertex_buffer_layout,
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.fs_shader.as_ref().unwrap(),
                entry_point: &self.fs_entry,
                targets: &[Some(init.config.format.into())],
            }),

            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                strip_index_format: self.strip_index_format,
                ..Default::default()
            },
            depth_stencil,
            multisample: wgpu::MultisampleState{
                count: init.sample_count,
                ..Default::default()
            },
            multiview: None,
        })
    }
}

impl IWgpuInit {
    pub async fn new(window: &Window, sample_count:u32, limits:Option<wgpu::Limits>) -> Self {
        let limits_device = limits.unwrap_or(wgpu::Limits::default());

        let size = window.inner_size();
        let instance = wgpu::Instance::default();

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    //features: wgpu::Features::empty(),
                    features:wgpu::Features::default() | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    //limits: wgpu::Limits::default(),
                    limits: limits_device
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode:surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
            size,
            sample_count,
        }
    }
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsCounter {
    // Creates a new FpsCounter.
    pub fn new() -> Self {
        Self {
            last_second_frames: VecDeque::with_capacity(128),
            last_print_time: Instant::now(),
        }
    }

    // updates the fps counter and print fps.
    pub fn print_fps(&mut self, interval:u64) {
        let now = Instant::now();
        let a_second_ago = now - Duration::from_secs(1);

        while self.last_second_frames.front().map_or(false, |t| *t < a_second_ago) {
            self.last_second_frames.pop_front();
        }

        self.last_second_frames.push_back(now);

        // Check if the interval seconds have passed since the last print time
        if now - self.last_print_time >= Duration::from_secs(interval) {
            let fps = self.last_second_frames.len();
            println!("FPS: {}", fps);
            self.last_print_time = now;
        }
    }
}

impl State {
    async fn new(
        window: &Window,

    ) -> Self {
        let init = IWgpuInit::new(&window,1, None).await;

        let shader = init
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // uniform data
        /*let model_mat = transforms::create_transforms(
            [-0.65 * width as f32, 5.0, -0.5 * height as f32],
            [0.0, 0.0, 0.0],
            [1.0, 100.0, 1.0],
        );*/
        let camera = CamPos{
            x:0.0,
            y:1000.0,
            z:0.0,
        };
        let camlook = CamPos{
            x:0.0,
            y:200.0,
            z:-30.0,
        };
        let mut terrain = surface::ITerrain::default();
        let mut translations: Vec<[f32; 2]> = vec![];
        let mut model_mat: Vec<[f32; 16]> = vec![];
        let chunk_size1 = (terrain.chunksize - 1) as f32;
        for i in 0..X_CHUNKS_COUNT {
            for j in 0..Z_CHUNKS_COUNT {
                let xt = -0.5 * X_CHUNKS_COUNT as f32 * chunk_size1 + i as f32 * chunk_size1;
                let zt = -0.5 * Z_CHUNKS_COUNT as f32 * chunk_size1 + j as f32 * chunk_size1;
                let translation = [xt, 10.0, zt];
                let m = transforms::create_transforms(translation, [0.0, 0.0, 0.0], [1.0, 150.0, 1.0]);
                model_mat.push(*(m.as_ref()));
                translations.push([xt, zt]);
            }
        }
        let model_storage_buffer =
            init.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Model Matrix Storage Buffer"),
                    contents: cast_slice(&model_mat),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let camera_position = (camera.x, camera.y, camera.z).into();
        let look_direction = (camlook.x, camlook.y, camlook.z).into();
        let up_direction = cgmath::Vector3::unit_y();

        let (view_mat, project_mat, vp_mat) = transforms::create_view_projection(
            camera_position,
            look_direction,
            up_direction,
            init.config.width as f32 / init.config.height as f32,
        );

        //let mvp_mat = vp_mat * model_mat;

        // create vertex uniform buffers
        let vertex_uniform_buffer = init.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Uniform Buffer"),
                    contents: cast_slice(vp_mat.as_ref() as &[f32; 16]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // uniform bind group for vertex shader
        let (vertex_bind_group_layout, vertex_bind_group) = create_bind_group_storage(
            &init.device,
            vec![wgpu::ShaderStages::VERTEX, wgpu::ShaderStages::VERTEX],
            vec![
                wgpu::BufferBindingType::Uniform,
                wgpu::BufferBindingType::Storage { read_only: true },
            ],
            &[
                vertex_uniform_buffer.as_entire_binding(),
                model_storage_buffer.as_entire_binding(),
            ],
        );

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: mem::size_of::<surface::Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3], // pos, col
        };

        let pipeline_layout = init
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&vertex_bind_group_layout],
                push_constant_ranges: &[],
            });

        let mut ppl = IRenderPipeline {
            shader: Some(&shader),
            pipeline_layout: Some(&pipeline_layout),
            vertex_buffer_layout: &[vertex_buffer_layout],
            ..Default::default()
        };
        let pipeline = ppl.new(&init);


        let depth_texture_view = create_depth_view(&init);
        let msaa_texture_view = create_msaa_texture_view(&init);
        let vertex_data = terrain.create_collection_of_terrain_data(
            X_CHUNKS_COUNT,
            Z_CHUNKS_COUNT,
            &translations,
        );
        let index_data = terrain.create_indices(vertex_data.1, vertex_data.1);
        let mut vertex_buffer: Vec<wgpu::Buffer> = vec![];
        let mut k: usize = 0;
        for _i in 0..X_CHUNKS_COUNT {
            for _j in 0..Z_CHUNKS_COUNT {
                let vb = init
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: cast_slice(&vertex_data.0[k]),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
                vertex_buffer.push(vb);
                k += 1;
            }
        }
        /*
        let vertex_buffer = init
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: cast_slice(&vertex_data.0),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        */
        let index_buffer = init
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: cast_slice(&index_data),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });


        Self {
            init,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_bind_group:vertex_bind_group,
            uniform_buffer:vertex_uniform_buffer,
            view_mat,
            project_mat,
            msaa_texture_view ,
            depth_texture_view,
            index_length: index_data.len() as u32,
            camera,
            camlook,
            translations,
            terrain,
            update_buffers: false,
            update_buffers_view: false,
            fps_counter: FpsCounter::default(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.init.size = new_size;
            self.init.config.width = new_size.width;
            self.init.config.height = new_size.height;
            self.init
                .surface
                .configure(&self.init.device, &self.init.config);

            self.project_mat =
                transforms::create_projection(new_size.width as f32 / new_size.height as f32, true);
            let vp_mat = self.project_mat * self.view_mat;
            self.init.queue.write_buffer(
                &self.uniform_buffer,
                0,
                cast_slice(vp_mat.as_ref() as &[f32; 16]),
            );

            self.depth_texture_view = create_depth_view(&self.init);
            if self.init.sample_count > 1 {
                self.msaa_texture_view = create_msaa_texture_view(&self.init);
            }
        }
    }

    pub fn plane_move(&mut self, moves: char) {
        match moves {
            's' => {/*
                if self.camlook.x < 120.0 {
                    self.camlook.x += 3.0;
                }*/
                    self.terrain.moves[0] += 10.0;
                    self.update_buffers = true;
            },
            'n' => {/*
                if self.camlook.x > -120.0 {
                    self.camlook.x -= 3.0;
                }*/
                    self.terrain.moves[0] -= 10.0;
                    self.update_buffers = true;

            },
            'w' => {/*if self.camlook.z == 30.0{if self.camlook.x<0.0{
                if self.camlook.x< -10.0{self.camlook.x += 3.0;}
                self.camlook.x += 1.0;
                }
                if self.camlook.x>0.0{
                if self.camlook.x> 10.0{self.camlook.x -= 3.0;}
                self.camlook.x -= 1.0;
                }}
                if self.camlook.z == -30.0{
                if self.camlook.x<=0.0 {
                if self.camlook.x > -120.0 {
                    self.camlook.x -= 3.0;
                }
                if self.camlook.x < -119.0{
                    self.camlook.z = 30.0;
                    self.camlook.x =-120.0;
                }}if self.camlook.x>0.0 {
                if self.camlook.x < 120.0 {
                    self.camlook.x += 3.0;
                }
                if self.camlook.x > 119.0 {
                    self.camlook.z = 30.0;
                    self.camlook.x = 120.0;
                }
                }}*/
                    self.terrain.moves[1] += 10.0;
                    self.update_buffers = true;

                },
            'e' =>{/*if self.camlook.z == 30.0{
                if self.camlook.x<=0.0 {
                if self.camlook.x > -120.0 {
                    self.camlook.x -= 3.0;
                }
                if self.camlook.x < -119.0{
                    self.camlook.z = -30.0;
                    self.camlook.x =-120.0;
                }}if self.camlook.x>0.0 {
                if self.camlook.x < 120.0 {
                    self.camlook.x += 3.0;
                }
                if self.camlook.x > 119.0 {
                    self.camlook.z = -30.0;
                    self.camlook.x = 120.0;
                }
                }}
                if self.camlook.z == -30.0{
                if self.camlook.x<0.0{
                if self.camlook.x< -10.0{self.camlook.x += 3.0;}
                self.camlook.x += 1.0;
                }
                if self.camlook.x>0.0{
                if self.camlook.x> 10.0{self.camlook.x -= 3.0;}
                self.camlook.x -= 1.0;
                }
            }*/
                    self.terrain.moves[1] -= 10.0;
                    self.update_buffers = true;

            },
            'u' => self.camera.y = self.camera.y+1.0,
            'd' => self.camera.y = self.camera.y-1.0,
            'r' => self.camlook.y = self.camlook.y+1.0,
            'f' => self.camlook.y = self.camlook.y-1.0,
            _ => {}
        }
        let look_direction = (self.camlook.x,self.camlook.y,self.camlook.z).into();
        let up_direction = cgmath::Vector3::unit_y();

        let camera_position = (self.camera.x, self.camera.y, self.camera.z).into();
        let (view_mat, project_mat, vp_mat) = transforms::create_view_projection(
            camera_position,
            look_direction,
            up_direction,
            self.init.config.width as f32 / self.init.config.height as f32,
        );

        self.view_mat=view_mat;
        self.project_mat=project_mat;
    }
    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                KeyboardInput {
                    virtual_keycode: Some(keycode),
                    state: ElementState::Pressed,
                    ..
                },
                ..
            } => match keycode {
                VirtualKeyCode::W => {
                    self.plane_move('e');
                    true
                }
                VirtualKeyCode::S => {
                    self.plane_move('w');
                    true
                }
                VirtualKeyCode::A => {
                    self.plane_move('n');
                    true
                }
                VirtualKeyCode::D => {
                    self.plane_move('s');
                    true
                }
                VirtualKeyCode::PageUp => {
                    self.plane_move('u');
                    self.update_buffers= true;
                    true
                }
                VirtualKeyCode::PageDown => {
                    self.plane_move('d');
                    self.update_buffers= true;
                    true
                }
                VirtualKeyCode::Q => {
                    self.plane_move('r');
                    self.update_buffers = true;
                    true
                }
                VirtualKeyCode::E => {
                    self.plane_move('f');
                    self.update_buffers = true;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn update(&mut self) {
        // update buffers:
        if self.update_buffers {
            let vertex_data = self.terrain.create_collection_of_terrain_data(
            X_CHUNKS_COUNT,
            Z_CHUNKS_COUNT,
            &self.translations,
        );
            let mut k = 0usize;
            for _i in 0..X_CHUNKS_COUNT {
                for _j in 0..Z_CHUNKS_COUNT {
                    self.init.queue.write_buffer(
                        &self.vertex_buffer[k],
                        0,
                        cast_slice(&vertex_data.0[k]),
                    );
                    k += 1;
                }
            }
            let vp_mat = self.project_mat * self.view_mat;
            self.init.queue.write_buffer(
                &self.uniform_buffer,
                0,
                cast_slice(vp_mat.as_ref() as &[f32; 16]),
            );
            let index_data = self.terrain.create_indices(vertex_data.1, vertex_data.1);
            self.init
                .queue
                .write_buffer(&self.index_buffer, 0, cast_slice(&index_data));
            self.index_length = index_data.len() as u32;
            self.update_buffers = false;
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        //let output = self.init.surface.get_current_frame()?.output;
        let output = self.init.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.init
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let color_attach = create_color_attachment(&view);
            let msaa_attach = create_msaa_color_attachment(&view, &self.msaa_texture_view);
            let color_attachment = if self.init.sample_count == 1 {
                color_attach
            } else {
                msaa_attach
            };
            let depth_attachment = create_depth_stencil_attachment(&self.depth_texture_view);

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(depth_attachment),
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                let mut k: u32 = 0;
                for _i in 0..X_CHUNKS_COUNT {
                    for _j in 0..Z_CHUNKS_COUNT {
                        render_pass.set_vertex_buffer(0, self.vertex_buffer[k as usize].slice(..));
                        render_pass.set_index_buffer(
                            self.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..self.index_length, 0, k..k + 1);
                        k += 1;
                    }
                }
        }

        self.fps_counter.print_fps(5);
        self.init.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}



pub fn create_bind_group_layout_storage(device: &wgpu::Device, shader_stages: Vec<wgpu::ShaderStages>, binding_types: Vec<wgpu::BufferBindingType>) -> wgpu::BindGroupLayout {
    let mut entries = vec![];

    for i in 0..shader_stages.len() {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: i as u32,
            visibility: shader_stages[i],
            ty: wgpu::BindingType::Buffer {
                ty: binding_types[i],
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        entries: &entries,
        label: Some("Uniform Bind Group Layout"),
    })
}

pub fn create_bind_group_storage(device: &wgpu::Device, shader_stages: Vec<wgpu::ShaderStages>, binding_types: Vec<wgpu::BufferBindingType>, resources: &[wgpu::BindingResource<'_>]) -> ( wgpu::BindGroupLayout, wgpu::BindGroup) {
    let entries: Vec<_> = resources.iter().enumerate().map(|(i, resource)| {
        wgpu::BindGroupEntry {
            binding: i as u32,
            resource: resource.clone(),
        }
    }).collect();

    let layout = create_bind_group_layout_storage(device, shader_stages, binding_types);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout,
        entries: &entries,
        label: Some("Uniform Bind Group"),
    });

    (layout, bind_group)
}

pub fn create_bind_group_layout(device: &wgpu::Device, shader_stages: Vec<wgpu::ShaderStages>) -> wgpu::BindGroupLayout {
    let mut entries = vec![];

    for i in 0..shader_stages.len() {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: i as u32,
            visibility: shader_stages[i],
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        entries: &entries,
        label: Some("Uniform Bind Group Layout"),
    })
}

pub fn create_bind_group(device: &wgpu::Device, shader_stages: Vec<wgpu::ShaderStages>, resources: &[wgpu::BindingResource<'_>]) -> ( wgpu::BindGroupLayout, wgpu::BindGroup) {
    let entries: Vec<_> = resources.iter().enumerate().map(|(i, resource)| {
        wgpu::BindGroupEntry {
            binding: i as u32,
            resource: resource.clone(),
        }
    }).collect();

    let layout = create_bind_group_layout(device, shader_stages);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout,
        entries: &entries,
        label: Some("Uniform Bind Group"),
    });

    (layout, bind_group)
}

pub fn create_color_attachment<'a>(texture_view: &'a wgpu::TextureView) -> wgpu::RenderPassColorAttachment<'a> {
    wgpu::RenderPassColorAttachment {
        view: texture_view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: true,
        },
    }
}

pub fn create_msaa_texture_view(init: &IWgpuInit) -> wgpu::TextureView{
    let msaa_texture = init.device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: init.config.width,
            height: init.config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: init.sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: init.config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    });

    msaa_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

pub fn create_msaa_color_attachment<'a>(texture_view: &'a wgpu::TextureView, msaa_view: &'a wgpu::TextureView)
-> wgpu::RenderPassColorAttachment<'a> {
    wgpu::RenderPassColorAttachment {
        view: msaa_view,
        resolve_target: Some(texture_view),
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: true,
        },
    }
}
pub fn create_depth_view(init: &IWgpuInit) -> wgpu::TextureView {
    let depth_texture = init.device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: init.config.width,
            height: init.config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: init.sample_count,
        dimension: wgpu::TextureDimension::D2,
        format:wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    });

    depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

pub fn create_depth_stencil_attachment<'a>(depth_view: &'a wgpu::TextureView) -> wgpu::RenderPassDepthStencilAttachment<'a> {
    wgpu::RenderPassDepthStencilAttachment {
        view: depth_view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: false,
        }),
        stencil_ops: None,
    }
}

pub fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    window.set_title(&*format!("Honours"));

    let mut state = pollster::block_on(State::new(
        &window,
    ));

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }

                    _ => {}
                }
            }
        }
        Event::RedrawRequested(_) => {
            state.update();

            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.init.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}