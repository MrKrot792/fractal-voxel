use std::ptr::null;
use std::{cell::RefCell, rc::Rc, sync::Arc};

mod fps;
use fps::Fps;

mod renderer;
use renderer::instance::InstanceManager;
use renderer::pipeline::{self, IndexBufferDescriptor, UniformDescriptor, VertexBufferDescriptor};

mod entities;
use entities::camera::{Camera, CameraDescriptor};
use entities::key_manager::KeyInputManager;

mod chunks;

use winit::{
  application::ApplicationHandler,
  event::*,
  event_loop::{ActiveEventLoop, EventLoop},
//keyboard::{KeyCode},
  window::Window,
};

// This will store the state of our game
pub struct State<'a> {
  instance: InstanceManager<'a>,
  fps: fps::Fps,
  _camera_id: usize,
  _key_manager_id: usize,
  chunk_manager: chunks::ChunkManager,
}

impl<'a> State<'a> {
  pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
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

    let mut chunk_manager = chunks::ChunkManager::new();
    chunks::Vertex::desc();
    chunk_manager.regenerate_chunks_at((0.0, 0.0, 0.0).into());
    let (vertices, indices) = chunk_manager.get_vertices_and_indices();

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
          contents: Some(Vec::from(bytemuck::cast_slice(&vertices))),
	  description: chunks::Vertex::desc(),
	},
      ],
      instance_buffer_index: None,
      instance_buffer_len:   None,
    };
    
    let render_pipeline_desc = pipeline::RenderPipelineManagerDescriptor {
      uniforms,
      vertex_buffers,
      instance_buffer_index: Some(1),
      index_buffer: IndexBufferDescriptor {
	contents: Some(Vec::from(bytemuck::cast_slice(&indices))),
	content_len: indices.len(),
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
      _key_manager_id: key_manager_id,
      chunk_manager,
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
