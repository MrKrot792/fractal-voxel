use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetFps {
  Value(u32),
  Unlimited
}

#[derive(Debug, Clone, Copy)]
pub struct Fps {
  frames_count: u32,
  elapsed: f64,
  last_frame: Instant,
  target_fps: TargetFps,
  pub fps: u32,
  pub fps_average: f64,
  pub delta: f64,
}

impl Fps {
  pub fn new(target_fps: TargetFps) -> Self {
    Self {
      frames_count: 0,
      elapsed: 0.0,
      last_frame: Instant::now(),
      target_fps,
      fps: 0,
      fps_average: 0.0,
      delta: 0.0,
    }
  }

  /// Call at the start of the frame (resets timer)
  pub fn frame_start(&mut self) {
    self.last_frame = Instant::now();
  }

  /// Call at the end of the frame
  pub fn frame_end(&mut self) {
    let now = Instant::now();

    self.delta = now.duration_since(self.last_frame).as_secs_f64();
    self.frames_count += 1;
    self.recalculate();
  }

  fn recalculate(&mut self) { 
    self.elapsed += self.delta;

    if self.elapsed >= 1.0 {
      self.elapsed = 0.0;
      self.fps = self.frames_count;
      self.frames_count = 0;
    }

    self.fps_average = if self.delta > 0.0 { 1.0 / self.delta } else { 0.0 };
  }
  
  pub fn sleep_till_end(&mut self) {
    // if let TargetFps::Value(v) = self.target_fps {
    //   use std::time::Duration;
    //   use std::thread::sleep;
    //   let frame_budget = 1.0 / v as f64;
    //   let elapsed_this_frame = self.last_frame.elapsed().as_secs_f64();
    //   let sleeping_time = frame_budget - elapsed_this_frame;
    //   if sleeping_time <= 0.0 { return; }
    //   sleep(Duration::from_secs_f64(sleeping_time));
    // }

    if let TargetFps::Value(v) = self.target_fps {
      use std::time::Duration;
      use std::thread::sleep;
      let frame_budget = 1.0 / v as f64;
      let sleeping_time = frame_budget - self.delta;
      if sleeping_time <= 0.0 { return; }
      sleep(Duration::from_secs_f64(sleeping_time));
      self.delta += sleeping_time;
      self.recalculate();
    }
  }
}
