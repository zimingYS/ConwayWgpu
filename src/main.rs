////项目说明:
////本项目实现使用Rust WGPU图像库编写一个康威生命游戏
////代码逻辑参考自https://linux.cn/article-8933-1.html
////此项目遵循GPL-v3.0进行开源
////如果此项目涉及侵权，请联系作者或在讨论中提出
////仅作学习探讨使用

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use wgpu::*;

fn main() {
    //初始化窗口
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    //窗口事件循环
    event_loop.run(move |event, _, control_flow|match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event{
            WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput{
                        state: ElementState::Pressed,
                        virtual_keycode:Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            }  => {
                *control_flow = ControlFlow::Exit
            }
            _ => {}
        },
        _ => {}
    });
}