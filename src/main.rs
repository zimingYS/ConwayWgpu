////项目说明:
////本项目实现使用Rust WGPU图像库编写一个康威生命游戏
////代码逻辑参考自https://linux.cn/article-8933-1.html
////此项目遵循GPL-v3.0进行开源
////如果此项目涉及侵权，请联系作者或在讨论中提出
////仅作学习探讨使用

use cgmath::{InnerSpace, Rotation3, Zero};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use wgpu::*;
use wgpu::util::DeviceExt;
use winit::window::Window;

//创建顶点
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.05,  0.05, 0.00], color: [1.0, 1.0, 1.0] },
    Vertex { position: [-0.05, -0.05, 0.00], color: [1.0, 1.0, 1.0] },
    Vertex { position: [ 0.05, -0.05, 0.00], color: [1.0, 1.0, 1.0] },
    Vertex { position: [ 0.05,  0.05, 0.00], color: [1.0, 1.0, 1.0] },
];

const INDICES: &[u16] = &[
    0,1,2,
    0,2,3,
];

impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                }
            ]
        }
    }
}




////实例化缓冲区
//定义实例化大小
const NUM_INSTANCES_PER_ROW: u32 = 20;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.00);

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        use std::mem;
        VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as BufferAddress,
            // 我们需要从把 Vertex 的 step mode 切换为 Instance
            // 这样着色器只有在开始处理一次新实例化绘制时，才会接受下一份实例
            step_mode: VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    // 虽然顶点着色器现在只使用位置 0 和 1，但在后面的教程中，我们将对 Vertex 使用位置 2、3 和 4
                    // 因此我们将从 5 号 slot 开始，以免在后面导致冲突
                    shader_location: 5,
                    format: VertexFormat::Float32x4,
                },
                // 一个 mat4 需要占用 4 个顶点 slot，因为严格来说它是 4 个vec4
                // 我们需要为每个 vec4 定义一个 slot，并在着色器中重新组装出 mat4
                VertexAttribute {
                    offset: size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 6,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 8]>() as BufferAddress,
                    shader_location: 7,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 12]>() as BufferAddress,
                    shader_location: 8,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}


struct State{
    //初始化部分
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    //窗口大小
    size: winit::dpi::PhysicalSize<u32>,
    //着色器
    render_pipeline: RenderPipeline,
    //顶点
    vertex_buffer: Buffer,
    //索引
    index_buffer: Buffer,
    num_indices: u32,
    //实例化
    instances: Vec<Instance>,
    instance_buffer: Buffer,
}
//用于处理一些操作
impl State{
    async fn new(window: &Window) -> Self{
        //设置窗口大小
        let size = window.inner_size();

    ////初始化窗口设置
        let instance = wgpu::Instance::new(Backends::all());
        let surface = unsafe {instance.create_surface(window)};
        let adapter = instance.request_adapter(
            &RequestAdapterOptions{
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }
        ).await.unwrap();

        let (device,queue) = adapter.request_device(
            &DeviceDescriptor{
                label: None,
                features: Features::empty(),
                limits: Limits::default(),
            },
            None,
        ).await.unwrap();

        let config = SurfaceConfiguration{
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };
        surface.configure(&device,&config);


    ////着色器设置部分
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main", // 1.
                buffers: &[
                    Vertex::desc(),
                    InstanceRaw::desc(),
                ], // 2.
            },
            fragment: Some(FragmentState { // 3.
                module: &shader,
                entry_point: "fs_main",
                targets: &[ColorTargetState { // 4.
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: FrontFace::Ccw, // 2.
                cull_mode: Some(Face::Back),
                // 如果将该字段设置为除了 Fill 之外的任何值，都需要 Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // 需要 Features::DEPTH_CLIP_ENABLE
                unclipped_depth: false,
                // 需要 Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
        });

        //顶点缓冲区
        let vertex_buffer = device.create_buffer_init(
            &util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: BufferUsages::VERTEX,
            }
        );

        //索引
        let index_buffer = device.create_buffer_init(
            &util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: BufferUsages::INDEX,
            }
        );
        let num_indices = INDICES.len() as u32;

        //实例化绘制
        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|y| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let position = cgmath::Vector3 {
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                    z: 0.00,
                } - INSTANCE_DISPLACEMENT;

                let position = position * 0.1;

                let rotation = if position.is_zero() {
                    // 需要这行特殊处理，这样在 (0, 0, 0) 的物体不会被缩放到 0
                    // 因为错误的四元数会影响到缩放
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
                };

                Instance {
                    position, rotation,
                }
            })
        }).collect::<Vec<_>>();

        //实例化缓冲
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: BufferUsages::VERTEX,
            }
        );

        State{
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            instances,
            instance_buffer,
        }
    }

    fn resize(&mut self, mut new_size: winit::dpi::PhysicalSize<u32>){
        let aspect_ratio:f32 = 1.0 / 1.0; //这里使用长宽比固定窗口比例

        let mut new_width = new_size.width;
        let mut new_height = new_size.height;

        if new_width as f32 / new_height as f32 > aspect_ratio {
            // 如果宽度/高度的比例大于 1:1，则按高度调整宽度
            new_width = (new_height as f32 * aspect_ratio) as u32;
        } else {
            // 如果宽度/高度的比例小于 1:1，则按宽度调整高度
            new_height = (new_width as f32 / aspect_ratio) as u32;
        }

        // 如果新的宽高大于 0，且符合固定的宽高比
        if new_width > 0 && new_height > 0 {
            self.size = winit::dpi::PhysicalSize::new(new_width, new_height);
            self.config.width = new_width;
            self.config.height = new_height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self,event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self){

    }

    fn render(&mut self) -> Result<(),SurfaceError>{
        //初始化部分
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        //背景调整部分
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            //此处调整背景色
                            r: 0.0, g: 0.0, b: 0.0, a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            //着色器绑定部分
            render_pass.set_pipeline(&self.render_pipeline);
            //顶点设置
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            //索引设置
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            //绘制
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
        }

        // submit 方法能传入任何实现了 IntoIter 的参数
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    //初始化窗口
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("WGPU Conway's Game of life")
        .with_inner_size(winit::dpi::PhysicalSize::new(500,500))
        .with_resizable(false)
        .build(&event_loop).unwrap();

    let mut state = pollster::block_on(State::new(&window));

    //窗口事件循环
    event_loop.run(move |event, _, control_flow|match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() =>if !state.input(event) {
            match event {
                WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                    input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => {
                    *control_flow = ControlFlow::Exit
                },
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                },
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    // new_inner_size 是 &&mut 类型，因此需要解引用两次
                    state.resize(**new_inner_size);
                }
                _ => {}
            }
        },
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // 如果发生上下文丢失，就重新配置 surface
                Err(SurfaceError::Lost) => state.resize(state.size),
                // 系统内存不足，此时应该退出
                Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // 所有其他错误（如过时、超时等）都应在下一帧解决
                Err(e) => eprintln!("{:?}", e),
            }
        },
        Event::MainEventsCleared => {
            // 除非手动请求，否则 RedrawRequested 只会触发一次
            window.request_redraw();
        }
        _ => {}
    });
}