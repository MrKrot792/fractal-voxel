use std::{collections::{HashMap, VecDeque}, ops::Add};

use cgmath::num_traits::ToPrimitive;

#[derive(Default, Copy, Clone, PartialEq)]
pub enum Block {
  Red,
  Green,
  Blue,
  #[default]
  Empty,
}

impl Block {
  pub fn get_color(&self) -> [u8; 4] {
    match *self {
      Block::Red =>   [255, 0, 0,   255],
      Block::Blue =>  [0,   0, 255, 255],
      Block::Green => [0, 255, 0,   255],
      Block::Empty => [255, 0, 255, 255], // purple
    }
  }
}

const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub struct Chunk {
  blocks: [Block; CHUNK_VOLUME],
}

impl Default for Chunk {
  fn default() -> Self {
    Self {
      blocks: [Block::default(); CHUNK_VOLUME],
    }
  }
}

#[derive(Default)]
pub struct ChunkGenerator {
  chunks_to_draw: VecDeque<cgmath::Vector3<i32>>,
  generated_chunks: HashMap<cgmath::Vector3<i32>, Chunk>,
  random: rand::rngs::ThreadRng,
}

// in blocks
const RENDERING_DISTANCE: i32 = 2;

impl ChunkGenerator {
  pub fn new() -> Self {
    Self::default()
  }

  fn get_position_from_index(index: usize) -> cgmath::Vector3<i32> {
    // TODO: z order algorithm
    return cgmath::Vector3::new(
      index % CHUNK_SIZE,
      (index / CHUNK_SIZE) % CHUNK_SIZE,
      index / (CHUNK_SIZE*CHUNK_SIZE),
    ).map(|x| x.try_into().unwrap())
  }

  fn generate_chunk(&mut self, position: &cgmath::Vector3<i32>) -> Chunk {
    let mut result: Chunk = Chunk::default();

    // for each block
    for (i, v) in &mut result.blocks.iter_mut().enumerate() {
      // cuz get_position_from_index is chunk local
      let block_position = Self::get_position_from_index(i) + (position * 16);

      if block_position.x == 0 {
	*v = Block::Red;
      }
      else if block_position.y == 0 {
	*v = Block::Green;
      }
      else if block_position.z == 0 {
	*v = Block::Blue;
      }
    }

    result
  }

  fn maybe_generate_chunk_at(&mut self, position: &cgmath::Vector3<i32>) {
    if !self.generated_chunks.contains_key(position) {
      let chunk = self.generate_chunk(position);
      self.generated_chunks.insert(*position, chunk);
    }
  }
  
  pub fn get_chunk_at(&mut self, position: cgmath::Vector3<i32>) -> &Chunk {
    self.maybe_generate_chunk_at(&position);
    self.generated_chunks.get(&position).unwrap()
  }

  pub fn get_chunks_from_player_position(&mut self, player_position: cgmath::Vector3<f32>) {
    let chunk_position: cgmath::Vector3<i32> =
      (player_position / 16.0)
      .map(|s| s.floor().to_i32().unwrap());

    let x_range = chunk_position.x as i32 - RENDERING_DISTANCE
      ..chunk_position.x as i32 + RENDERING_DISTANCE;
    let y_range = chunk_position.y as i32 - RENDERING_DISTANCE
      ..chunk_position.y as i32 + RENDERING_DISTANCE;
    let z_range = chunk_position.z as i32 - RENDERING_DISTANCE
      ..chunk_position.z as i32 + RENDERING_DISTANCE;

    for x in x_range.clone() {
      for y in y_range.clone() {
        for z in z_range.clone() {
          self.chunks_to_draw.push_back((x, y, z).into());
        }
      }
    }

    println!(
      "X: {}, Y: {}, Z: {}",
      x_range.len(),
      y_range.len(),
      z_range.len()
    );
  }
}

const CUBE_VERTICES: &[cgmath::Vector3<i32>] = &[
  cgmath::Vector3::new(1, 1, 0), cgmath::Vector3::new(1, 0, 0), cgmath::Vector3::new(1, 1, 1), cgmath::Vector3::new(1, 0, 1), cgmath::Vector3::new(0, 1, 0), cgmath::Vector3::new(0, 0, 0), cgmath::Vector3::new(0, 1, 1), cgmath::Vector3::new(0, 0, 1),
];

// in groups of 3
const CUBE_INDICES: &[u32] = &[
  0, 6, 2, 3, 6, 7, 7, 4, 5, 5, 3, 7, 1, 2, 3, 5, 0, 1, 0, 4, 6, 3, 2, 6, 7, 6, 4, 5, 1, 3, 1, 0, 2, 5, 4, 0,
];

#[repr(C)]
#[derive(Eq, Hash, PartialEq, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
  pub position: [i32; 3],
  pub color: [u8; 4],
}

impl Vertex {
  const ATTRIBS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Sint32x3, 1 => Unorm8x4];

  pub fn new(position: [i32; 3], color: [u8; 4]) -> Self {
    Self {
      position,
      color,
    }
  }

  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    use std::mem;

    wgpu::VertexBufferLayout {
      array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &Self::ATTRIBS,
    }
  }
}

#[derive(Default)]
pub struct ChunkVerticesGenerator {
  pub vertices: Vec<Vertex>,
  vertices_hashmap: HashMap<Vertex, u32>,
  pub indices: Vec<u32>,
}

impl ChunkVerticesGenerator {
  pub fn new() -> Self {
    Self::default()
  }

  // TODO: clean this shit up
  // TODO: another meshing algorithm
  pub fn regenerate_vertices(
    &mut self,
    chunks: &mut ChunkGenerator,
  ) {
    self.indices.clear();
    self.vertices.clear();
    self.vertices_hashmap.clear();

    let chunks_to_draw: Vec<_> = chunks.chunks_to_draw.drain(..).collect();

    println!("Chunks to draw size: {}", chunks_to_draw.len());
    println!("by chunk_volume: {}", CHUNK_VOLUME);
    println!("by cube indices: {}", CUBE_INDICES.len());
    println!("So the result is: {}", chunks_to_draw.len() * CHUNK_VOLUME * CUBE_INDICES.len());

    let mut vertex_count = 0;
    
    for current_chunk_position in chunks_to_draw {
      let current_chunk = chunks.get_chunk_at(current_chunk_position);
      println!("current_chunk_position: {:?}", current_chunk_position);
	
      for (block_index, block_type) in current_chunk.blocks.iter().enumerate() {
	if *block_type == Block::Empty { continue; }
	let global_block_position =
	  ChunkGenerator::get_position_from_index(block_index)
	  .add(current_chunk_position * CHUNK_SIZE as i32);

	for index in CUBE_INDICES {
	  let vertex = Vertex::new((CUBE_VERTICES[*index as usize] + global_block_position).into(), block_type.get_color());
	  if !self.vertices_hashmap.contains_key(&vertex) {
	    self.vertices_hashmap.insert(vertex, self.vertices.len() as u32);
	    self.indices.push(self.vertices.len() as u32);
	    self.vertices.push(vertex);
	  } else {
	    self.indices.push(self.vertices_hashmap[&vertex]);
	  }
	  vertex_count += 1;
	}
      }
    }

    println!("--------------------");
    println!("Going to actually draw {} vertices, {} triangles", vertex_count, vertex_count / 3);
  }
}

#[derive(Default)]
pub struct ChunkManager {
  pub chunk_generator: ChunkGenerator,
  pub chunk_vertices_generator: ChunkVerticesGenerator,
}

impl ChunkManager {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn regenerate_chunks_at(&mut self, position: cgmath::Vector3<f32>) {
    self.chunk_generator.get_chunks_from_player_position(position);
    self.chunk_vertices_generator.regenerate_vertices(
      &mut self.chunk_generator
    );
  }

  pub fn get_vertices_and_indices(&self) -> (&Vec<Vertex>, &Vec<u32>) {
    (&self.chunk_vertices_generator.vertices, &self.chunk_vertices_generator.indices)
  }
}
