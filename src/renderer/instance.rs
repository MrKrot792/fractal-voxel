use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;
use crate::entities::entity;
use crate::renderer::pipeline::{
  RenderPipelineManager,
  RenderPipelineManagerDescriptor
};

pub struct DepthTexture {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  entity_id: usize,
}

impl DepthTexture {
  pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
    let t = Self::create_texture_fn(device, config);
    Self {
      view: Self::create_texture_view_fn(&t),
      texture: t,
      entity_id: 0,
    }
  }

  pub fn manage(s: Rc<RefCell<DepthTexture>>, entity_manager: &mut entity::EntityManager) -> usize {
    entity_manager.entity_create(s)
  }
  
  fn create_texture_fn(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Depth32Float,
      mip_level_count: 1,
      sample_count: 1,
      size: wgpu::Extent3d {
	width:  config.width,
	height: config.height,
	depth_or_array_layers: 1,
      },
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
      view_formats: &[wgpu::TextureFormat::Depth32Float],
    })
  }

  fn create_texture_view_fn(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
      label: None,
      format: None,
      dimension: None,
      usage: None,
      aspect: wgpu::TextureAspect::DepthOnly,
      base_mip_level: 0,
      mip_level_count: None,
      base_array_layer: 0,
      array_layer_count: None,
    })
  }

  fn recreate_texture_and_view(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
    self.texture.destroy();
    self.texture = Self::create_texture_fn(device, config);
    self.view = Self::create_texture_view_fn(&self.texture);
  }
}

impl entity::Entity for DepthTexture {
  fn set_id(&mut self, new_id: usize) {
    self.entity_id = new_id;
  }

  fn event(&mut self, _entity_index: &usize,  render_context: &mut self::RenderContext, event: &entity::Event) -> anyhow::Result<()> {
    match *event {
      entity::Event::Resized(_) => self.recreate_texture_and_view(&render_context.device, &render_context.config),
      _ => (),
    }
    
    Ok(())
  }
}

pub struct RenderContext<'a> {
  surface: wgpu::Surface<'a>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  is_surface_configured: bool,
  window: Arc<Window>,
  render_pipeline: RenderPipelineManager<'a>,
  depth_texture: Rc<RefCell<DepthTexture>>,
}

impl<'a> RenderContext<'a> {
  pub fn get_config(&self) -> &wgpu::SurfaceConfiguration {
    &self.config
  }

  pub fn get_config_mut(&mut self) -> &mut wgpu::SurfaceConfiguration {
    &mut self.config
  }

  pub fn uniform_write(&mut self, buf: &[u8], buf_index: usize) {
    self.render_pipeline.uniforms.write(&self.queue, buf, buf_index);
  }
  
  pub fn configure_surface(&mut self) {
    self.surface.configure(&self.device, &self.config);
    self.is_surface_configured = true;
  }

  fn get_output(&mut self, is_suboptimal: &mut bool) -> Option<wgpu::SurfaceTexture> {
    let output = match self.surface.get_current_texture() {
      wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
      wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
	*is_suboptimal = true;
	t
      },
      wgpu::CurrentSurfaceTexture::Timeout
	| wgpu::CurrentSurfaceTexture::Occluded
	| wgpu::CurrentSurfaceTexture::Validation => {
          // Skip this frame
          return None;
	}
      wgpu::CurrentSurfaceTexture::Outdated => {
	self.surface.configure(&self.device, &self.config);
	return None;
      }
      wgpu::CurrentSurfaceTexture::Lost => {
	panic!("Lost device");
      }
    };

    Some(output)
  }
  
  pub fn render(&mut self) -> anyhow::Result<()> {
    self.window.request_redraw();

    if !self.is_surface_configured {
      return Ok(());
    }

    let mut is_suboptimal = false;
    let output = match self.get_output(&mut is_suboptimal) {
      None => return Ok(()),
      Some(o) => o,
    };
    
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Render Encoder"),
    });
    
    {
      let depth_texture = self.depth_texture.borrow_mut();
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[
	  Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Clear(wgpu::Color {
		r: 0.1,
		g: 0.2,
		b: 0.3,
		a: 1.0,
              }),
              store: wgpu::StoreOp::Store,
            },
          }),
	],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
	  view: &depth_texture.view,
	  depth_ops: Some(wgpu::Operations {
	    load: wgpu::LoadOp::Clear(1.0),
	    store: wgpu::StoreOp::Store,
	  }),
	  stencil_ops: None,
	}),
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
      });

      self.render_pipeline.draw(&mut render_pass);
    }

    // submit will accept anything that implements IntoIter
    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    
    if is_suboptimal {
      self.surface.configure(&self.device, &self.config);
    }
    
    Ok(())
  }
}

pub struct InstanceManager<'a> {
  pub render_context: RenderContext<'a>,
  pub entity_manager: entity::EntityManager,
}

impl<'a> InstanceManager<'a> {
  pub async fn new(window: Arc<Window>, render_pipeline_desc: RenderPipelineManagerDescriptor<'a>) -> anyhow::Result<Self> {
    let size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
      backends: wgpu::Backends::PRIMARY,
      flags: Default::default(),
      memory_budget_thresholds: Default::default(),
      backend_options: Default::default(),
      display: None,
    });

    let surface = instance.create_surface(window.clone()).unwrap();

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
      })
      .await?;

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
      label: None,
      required_features: wgpu::Features::empty(),
      experimental_features: wgpu::ExperimentalFeatures::disabled(),
      required_limits: wgpu::Limits::default(),
      memory_hints: Default::default(),
      trace: wgpu::Trace::Off,
    }).await?;

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats.iter()
      .find(|f| f.is_srgb())
      .copied()
      .unwrap_or(surface_caps.formats[0]);
    
    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface_format,
      width: size.width,
      height: size.height,
      present_mode: surface_caps.present_modes[0],
      alpha_mode: surface_caps.alpha_modes[0],
      view_formats: vec![],
      desired_maximum_frame_latency: 2,
    };

    let render_pipeline = RenderPipelineManager::new(
      render_pipeline_desc,
      &device,
      &config
    );

    let mut entity_manager = entity::EntityManager::new();

    let depth_texture = Rc::new(RefCell::new(DepthTexture::new(&device, &config)));
    DepthTexture::manage(depth_texture.clone(), &mut entity_manager);

    Ok(Self {
      render_context: RenderContext {
	surface,
	device,
	queue,
	config,
	is_surface_configured: false,
	window,
	render_pipeline,
	depth_texture,
      },
      entity_manager,
    })
  }
}
