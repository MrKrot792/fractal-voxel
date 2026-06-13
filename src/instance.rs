use std::sync::Arc;
use winit::window::Window;
use crate::pipeline::{
  RenderPipelineManager,
  RenderPipelineManagerDescriptor,
};

pub struct InstanceManager<'a> {
  surface: wgpu::Surface<'a>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  is_surface_configured: bool,
  window: Arc<Window>,
  render_pipeline: RenderPipelineManager<'a>,
}

impl<'a> InstanceManager<'a> {
  pub fn get_config(&self) -> &wgpu::SurfaceConfiguration {
    &self.config
  }

  pub fn get_config_mut(&mut self) -> &mut wgpu::SurfaceConfiguration {
    &mut self.config
  }

  pub fn uniform_write(&mut self, buf: &[u8], buf_index: usize) {
    self.render_pipeline.uniforms.write(&self.queue, buf, buf_index);
  }
  
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
    
    Ok(Self {
      surface,
      device,
      queue,
      config,
      is_surface_configured: false,
      window,
      render_pipeline
    })
  }

  pub fn configure_surface(&mut self) {
    self.surface.configure(&self.device, &self.config);
    self.is_surface_configured = true;
  }

  pub fn render(&mut self) -> anyhow::Result<()> {
    self.window.request_redraw();

    if !self.is_surface_configured {
      return Ok(());
    }

    let mut is_suboptimal = false;
    let output = match self.surface.get_current_texture() {
      wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
      wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
	is_suboptimal = true;
	t
      },
      wgpu::CurrentSurfaceTexture::Timeout
	| wgpu::CurrentSurfaceTexture::Occluded
	| wgpu::CurrentSurfaceTexture::Validation => {
          // Skip this frame
          return Ok(());
	}
      wgpu::CurrentSurfaceTexture::Outdated => {
	self.surface.configure(&self.device, &self.config);
	return Ok(());
      }
      wgpu::CurrentSurfaceTexture::Lost => {
	// You could recreate the devices and all resources
	// created with it here, but we'll just bail
	anyhow::bail!("Lost device");
      }
    };

    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Render Encoder"),
    });

    {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
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
        })],
        depth_stencil_attachment: None,
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
