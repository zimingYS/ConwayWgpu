////项目说明:
////本项目实现使用Rust WGPU图像库编写一个康威生命游戏
////代码逻辑参考自https://linux.cn/article-8933-1.html
////此项目遵循GPL-v3.0进行开源
////如果此项目涉及侵权，请联系作者或在讨论中提出
////仅作学习探讨使用



//创建顶点
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
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

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use wgpu::*;
use wgpu::util::DeviceExt;
use winit::window::Window;

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
    num_vertices: u32,
}
//用于处理一些操作
impl State{
    async fn new(window: &Window) -> Self{
        //设置窗口大小
        let size = window.inner_size();

    ////初始化窗口设置
        let instance = Instance::new(Backends::all());
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

        //顶点计数
        let num_vertices = VERTICES.len() as u32;

        State{
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            num_vertices,
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
            //顶点绘制
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
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