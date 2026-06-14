use std::{cell::RefCell, rc::Rc, sync::Arc};

mod fps;
use fps::Fps;

mod renderer;
use renderer::instance::InstanceManager;
use renderer::pipeline::{self, IndexBufferDescriptor, UniformDescriptor, VertexBufferDescriptor};

mod entities;
use entities::camera::{Camera, CameraDescriptor};
use entities::key_manager::KeyInputManager;

use winit::{
  application::ApplicationHandler,
  event::*,
  event_loop::{ActiveEventLoop, EventLoop},
//keyboard::{KeyCode},
  window::Window,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
  position: [f32; 3],
  color: [f32; 3],
}

const VERTICES: &[Vertex] = &[
  Vertex { position: [ 1.0,  1.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 1.0,  0.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 1.0,  1.0,  1.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 1.0,  0.0,  1.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  1.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  0.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  1.0,  1.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  0.0,  1.0], color: [0.0, 0.0, 0.0] },
];

const INDICES: &[u16] = &[
  4, 2, 0,
  2, 7, 3,
  6, 5, 7,
  1, 7, 5,
  0, 3, 1,
  4, 1, 5,
  4, 6, 2,
  2, 6, 7,
  6, 4, 5,
  1, 3, 7,
  0, 2, 3,
  4, 0, 1,
];

impl Vertex {
  const ATTRIBS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

  fn desc() -> wgpu::VertexBufferLayout<'static> {
    use std::mem;

    wgpu::VertexBufferLayout {
      array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &Self::ATTRIBS,
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexInstance { 
  position: [f32; 3],
  color: [f32; 3],
}

impl VertexInstance {
  const ATTRIBS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![2 => Float32x3, 3 => Float32x3];

  fn desc() -> wgpu::VertexBufferLayout<'static> {
    use std::mem;
    
    wgpu::VertexBufferLayout {
      array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &Self::ATTRIBS,
    }
  }
}

// This will store the state of our game
pub struct State<'a> {
  instance: InstanceManager<'a>,
  fps: fps::Fps,
  _camera_id: usize,
  _key_manager_id: usize,
}

impl<'a> State<'a> {
  pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
    let mut instances: Vec<VertexInstance> = Vec::new();
    for i in 0..8 {
      for j in 0..8 {
	for k in 0..8 {
	  instances.push(VertexInstance {
	    position: [i as f32, j as f32, k as f32],
	    color: [i as f32 / 8.0, j as f32 / 8.0, k as f32 / 8.0],
	  });
	}
      }
    }
    
    let fps = Fps::new(fps::TargetFps::Value(60));
    let size = window.inner_size();
    let camera = Rc::new(RefCell::new(Camera::new(
      CameraDescriptor {
	speed: 5.0,
	sensitivity: 0.001,
	fovy: 90.0,
	aspect: size.width as f32 / size.height as f32,
	fps,
      }
    )));

    {
      camera.borrow_mut().update_view_proj();
    }

    let view_proj = {
      let camera_b = camera.borrow();
      camera_b.get_view_proj()
    };
    
    let uniforms = vec![UniformDescriptor {
      contents: bytemuck::cast_slice(&[view_proj]).into(),
      visibility: wgpu::ShaderStages::VERTEX,
    }];

    let vertex_buffers = pipeline::VertexBuffersDescriptor {
      buffers: vec![
	VertexBufferDescriptor {
          contents: bytemuck::cast_slice(VERTICES).into(),
	  description: Vertex::desc(),
	},
	VertexBufferDescriptor {
          contents: bytemuck::cast_slice(instances.clone().as_slice()).into(),
	  description: VertexInstance::desc(),
	},
      ],
      instance_buffer_index: Some(1),
      instance_buffer_len: Some(instances.len()),
    };
    
    let render_pipeline_desc = pipeline::RenderPipelineManagerDescriptor {
      uniforms,
      vertex_buffers,
      instance_buffer_index: Some(1),
      index_buffer: IndexBufferDescriptor {
	contents: bytemuck::cast_slice(INDICES).into(),
	content_len: INDICES.len(),
      },
      shader: pipeline::ShaderDataDescriptor::RawData(include_str!("shader.wgsl")),
    };

    let mut instance_manager = InstanceManager::new(window, render_pipeline_desc).await?;

    let mut key_manager = KeyInputManager::new();
    key_manager.register(camera.clone());
    let camera_id = Camera::manage(
      camera,
      &mut instance_manager.entity_manager
    );
    let key_manager_id = key_manager.manage(
      &mut instance_manager.entity_manager
    );
    Ok(Self {
      instance: instance_manager,
      fps,
      _camera_id: camera_id,
      _key_manager_id: key_manager_id
    })
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    if width > 0 && height > 0 {
      let config = self.instance.render_context.get_config_mut();
      config.width = width;
      config.height = height;

      self.instance.render_context.configure_surface();
    }
  }

  fn render(&mut self) -> anyhow::Result<()> {
    self.instance.render_context.render()
  }

  fn update(&mut self) -> anyhow::Result<()> {
    self.instance.entity_manager.update(
      &mut self.instance.render_context,
      &self.fps
    )?;
    
    dbg!(self.fps);

    Ok(())
  }
}

pub struct App {
  state: Option<State<'static>>,
}

impl App {
  pub fn new() -> Self {
    Self {
      state: None,
    }
  }
}

impl ApplicationHandler<State<'static>> for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window_attributes = Window::default_attributes()
      .with_title("VoxelGame");

    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
    window.set_cursor_grab(winit::window::CursorGrabMode::Locked).unwrap();
    window.set_cursor_visible(false);
    self.state = Some(pollster::block_on(State::new(window)).unwrap());
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    let state = match &mut self.state {
      Some(canvas) => canvas,
      None => return,
    };

    match event {
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::Resized(size) => {
	state.resize(size.width, size.height);
	state.instance.entity_manager.handle_event_window(event);
      }
      WindowEvent::RedrawRequested => {
	state.fps.frame_start();
	state.instance.entity_manager.dispatch_events(&mut state.instance.render_context).unwrap();
	state.update().unwrap();
	
	match state.render() {
	  Ok(_) => {}
	  Err(e) => {
	    log::error!("{e}");
	    event_loop.exit();
	  }
	}

	state.fps.frame_end();	
	state.fps.sleep_till_end();
      },
      _ => state.instance.entity_manager.handle_event_window(event),
    }
  }

  fn device_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    _device_id: DeviceId,
    event: DeviceEvent,
  ) {
    let state = match &mut self.state {
      Some(canvas) => canvas,
      None => return,
    };
    state.instance.entity_manager.handle_event_device(event);
  }
}

pub fn run() -> anyhow::Result<()> {
  env_logger::init();

  let event_loop = EventLoop::with_user_event().build()?;
  let mut app = App::new();
  event_loop.run_app(&mut app)?;
  Ok(())
}
