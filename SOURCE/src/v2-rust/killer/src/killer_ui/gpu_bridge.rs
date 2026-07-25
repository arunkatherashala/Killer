//! **GPU Bridge** — GPU rendering abstraction layer.
//!
//! Records GPU commands (draw calls, state changes, shader binds) into a
//! command buffer that can be replayed on WebGL, WebGPU, Vulkan, or Metal.
//! Pure Rust state machine — no actual GPU API calls (those are backend-specific).


use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// GPU Types
// ══════════════════════════════════════════════════════════════════════════════

pub type GpuHandle = u64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuPrimitive {
    Triangles,
    TriangleStrip,
    TriangleFan,
    Lines,
    LineStrip,
    Points,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    Uint8x4,
    Uint16,
    Uint32,
}

impl GpuFormat {
    pub fn byte_size(&self) -> usize {
        match self {
            GpuFormat::Float32 => 4,
            GpuFormat::Float32x2 => 8,
            GpuFormat::Float32x3 => 12,
            GpuFormat::Float32x4 => 16,
            GpuFormat::Uint8x4 => 4,
            GpuFormat::Uint16 => 2,
            GpuFormat::Uint32 => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Opaque,
    AlphaBlend,
    Additive,
    Premultiplied,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CullMode {
    None,
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DepthCompare {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    GreaterEqual,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureFormat {
    Rgba8,
    Rgba16Float,
    Rgba32Float,
    Depth24,
    Depth32Float,
    Bc1, // DXT1
    Bc3, // DXT5
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureFilter {
    Nearest,
    Linear,
    MipmapNearest,
    MipmapLinear,
}

// ══════════════════════════════════════════════════════════════════════════════
// GPU Resources
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct GpuBuffer {
    pub handle: GpuHandle,
    pub size_bytes: usize,
    pub usage: BufferUsage,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    Storage,
}

#[derive(Debug, Clone)]
pub struct GpuTexture {
    pub handle: GpuHandle,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mip_levels: u32,
}

#[derive(Debug, Clone)]
pub struct GpuShader {
    pub handle: GpuHandle,
    pub vertex_source: String,
    pub fragment_source: String,
    pub name: String,
}

/// Vertex attribute layout descriptor.
#[derive(Debug, Clone)]
pub struct VertexAttribute {
    pub location: u32,
    pub format: GpuFormat,
    pub offset: usize,
    pub name: String,
}

/// Pipeline state.
#[derive(Debug, Clone)]
pub struct GpuPipeline {
    pub handle: GpuHandle,
    pub shader: GpuHandle,
    pub vertex_layout: Vec<VertexAttribute>,
    pub stride: usize,
    pub blend: BlendMode,
    pub cull: CullMode,
    pub depth_compare: DepthCompare,
    pub depth_write: bool,
    pub primitive: GpuPrimitive,
}

// ══════════════════════════════════════════════════════════════════════════════
// GPU Commands (command buffer)
// ══════════════════════════════════════════════════════════════════════════════

/// A single GPU command.
#[derive(Debug, Clone)]
pub enum GpuCommand {
    SetViewport { x: u32, y: u32, width: u32, height: u32 },
    SetScissor { x: u32, y: u32, width: u32, height: u32 },
    ClearColor { r: f32, g: f32, b: f32, a: f32 },
    ClearDepth { value: f32 },
    BindPipeline { pipeline: GpuHandle },
    BindVertexBuffer { buffer: GpuHandle, slot: u32 },
    BindIndexBuffer { buffer: GpuHandle, format: GpuFormat },
    BindTexture { texture: GpuHandle, slot: u32 },
    SetUniformMat4 { location: u32, value: [f32; 16] },
    SetUniformVec3 { location: u32, value: [f32; 3] },
    SetUniformVec4 { location: u32, value: [f32; 4] },
    SetUniformFloat { location: u32, value: f32 },
    SetUniformInt { location: u32, value: i32 },
    Draw { vertex_count: u32, first_vertex: u32 },
    DrawIndexed { index_count: u32, first_index: u32, base_vertex: i32 },
    DrawInstanced { vertex_count: u32, instance_count: u32 },
    DrawIndexedInstanced { index_count: u32, instance_count: u32 },
    BeginRenderPass { color_target: GpuHandle, depth_target: Option<GpuHandle> },
    EndRenderPass,
    CopyBufferToTexture { buffer: GpuHandle, texture: GpuHandle },
    GenerateMipmaps { texture: GpuHandle },
}

/// Command buffer — records GPU commands for deferred execution.
#[derive(Debug, Clone)]
pub struct GpuCommandBuffer {
    pub commands: Vec<GpuCommand>,
    pub label: String,
}

impl GpuCommandBuffer {
    pub fn new(label: &str) -> Self {
        GpuCommandBuffer { commands: Vec::new(), label: label.into() }
    }

    pub fn push(&mut self, cmd: GpuCommand) { self.commands.push(cmd); }

    pub fn clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.push(GpuCommand::ClearColor { r, g, b, a });
    }

    pub fn clear_depth(&mut self, value: f32) {
        self.push(GpuCommand::ClearDepth { value });
    }

    pub fn set_viewport(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.push(GpuCommand::SetViewport { x, y, width: w, height: h });
    }

    pub fn bind_pipeline(&mut self, pipeline: GpuHandle) {
        self.push(GpuCommand::BindPipeline { pipeline });
    }

    pub fn bind_vertex_buffer(&mut self, buffer: GpuHandle, slot: u32) {
        self.push(GpuCommand::BindVertexBuffer { buffer, slot });
    }

    pub fn bind_index_buffer(&mut self, buffer: GpuHandle) {
        self.push(GpuCommand::BindIndexBuffer { buffer, format: GpuFormat::Uint32 });
    }

    pub fn bind_texture(&mut self, texture: GpuHandle, slot: u32) {
        self.push(GpuCommand::BindTexture { texture, slot });
    }

    pub fn draw(&mut self, vertex_count: u32) {
        self.push(GpuCommand::Draw { vertex_count, first_vertex: 0 });
    }

    pub fn draw_indexed(&mut self, index_count: u32) {
        self.push(GpuCommand::DrawIndexed { index_count, first_index: 0, base_vertex: 0 });
    }

    pub fn draw_instanced(&mut self, vertex_count: u32, instances: u32) {
        self.push(GpuCommand::DrawInstanced { vertex_count, instance_count: instances });
    }

    pub fn command_count(&self) -> usize { self.commands.len() }
}

// ══════════════════════════════════════════════════════════════════════════════
// GPU Device (resource manager)
// ══════════════════════════════════════════════════════════════════════════════

/// Virtual GPU device — manages resource allocation and command recording.
pub struct GpuDevice {
    next_handle: GpuHandle,
    buffers: HashMap<GpuHandle, GpuBuffer>,
    textures: HashMap<GpuHandle, GpuTexture>,
    shaders: HashMap<GpuHandle, GpuShader>,
    pipelines: HashMap<GpuHandle, GpuPipeline>,
    pub backend: GpuBackend,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuBackend {
    WebGL2,
    WebGPU,
    Vulkan,
    Metal,
    Software,
}

impl GpuDevice {
    pub fn new(backend: GpuBackend) -> Self {
        GpuDevice {
            next_handle: 1,
            buffers: HashMap::new(),
            textures: HashMap::new(),
            shaders: HashMap::new(),
            pipelines: HashMap::new(),
            backend,
        }
    }

    fn alloc_handle(&mut self) -> GpuHandle {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }

    pub fn create_buffer(&mut self, size: usize, usage: BufferUsage) -> GpuHandle {
        let h = self.alloc_handle();
        self.buffers.insert(h, GpuBuffer { handle: h, size_bytes: size, usage });
        h
    }

    pub fn create_texture(&mut self, width: u32, height: u32, format: TextureFormat) -> GpuHandle {
        let h = self.alloc_handle();
        let mip_levels = (width.max(height) as f64).log2().floor() as u32 + 1;
        self.textures.insert(h, GpuTexture { handle: h, width, height, format, mip_levels });
        h
    }

    pub fn create_shader(&mut self, name: &str, vertex_src: &str, fragment_src: &str) -> GpuHandle {
        let h = self.alloc_handle();
        self.shaders.insert(h, GpuShader {
            handle: h, vertex_source: vertex_src.into(), fragment_source: fragment_src.into(), name: name.into(),
        });
        h
    }

    pub fn create_pipeline(&mut self, shader: GpuHandle, layout: Vec<VertexAttribute>, stride: usize) -> GpuHandle {
        let h = self.alloc_handle();
        self.pipelines.insert(h, GpuPipeline {
            handle: h, shader, vertex_layout: layout, stride,
            blend: BlendMode::Opaque, cull: CullMode::Back,
            depth_compare: DepthCompare::Less, depth_write: true,
            primitive: GpuPrimitive::Triangles,
        });
        h
    }

    pub fn destroy_buffer(&mut self, handle: GpuHandle) { self.buffers.remove(&handle); }
    pub fn destroy_texture(&mut self, handle: GpuHandle) { self.textures.remove(&handle); }

    pub fn buffer_count(&self) -> usize { self.buffers.len() }
    pub fn texture_count(&self) -> usize { self.textures.len() }
    pub fn shader_count(&self) -> usize { self.shaders.len() }
    pub fn pipeline_count(&self) -> usize { self.pipelines.len() }

    /// Create a new command buffer.
    pub fn create_command_buffer(&self, label: &str) -> GpuCommandBuffer {
        GpuCommandBuffer::new(label)
    }

    /// Estimate total GPU memory usage.
    pub fn estimated_memory(&self) -> usize {
        let buf_mem: usize = self.buffers.values().map(|b| b.size_bytes).sum();
        let tex_mem: usize = self.textures.values().map(|t| {
            let bpp = match t.format {
                TextureFormat::Rgba8 => 4,
                TextureFormat::Rgba16Float => 8,
                TextureFormat::Rgba32Float => 16,
                TextureFormat::Depth24 => 3,
                TextureFormat::Depth32Float => 4,
                TextureFormat::Bc1 => 1, // compressed
                TextureFormat::Bc3 => 1,
            };
            (t.width as usize) * (t.height as usize) * bpp
        }).sum();
        buf_mem + tex_mem
    }
}

impl Default for GpuDevice {
    fn default() -> Self { Self::new(GpuBackend::Software) }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_device() {
        let dev = GpuDevice::new(GpuBackend::WebGPU);
        assert_eq!(dev.backend, GpuBackend::WebGPU);
        assert_eq!(dev.buffer_count(), 0);
    }

    #[test]
    fn create_buffer() {
        let mut dev = GpuDevice::default();
        let vbo = dev.create_buffer(1024, BufferUsage::Vertex);
        let ibo = dev.create_buffer(256, BufferUsage::Index);
        assert_eq!(dev.buffer_count(), 2);
        assert_ne!(vbo, ibo);
    }

    #[test]
    fn create_texture() {
        let mut dev = GpuDevice::default();
        let _tex = dev.create_texture(512, 512, TextureFormat::Rgba8);
        assert_eq!(dev.texture_count(), 1);
        assert!(dev.estimated_memory() > 0);
    }

    #[test]
    fn create_pipeline() {
        let mut dev = GpuDevice::default();
        let shader = dev.create_shader("basic", "void main(){}", "void main(){}");
        let layout = vec![
            VertexAttribute { location: 0, format: GpuFormat::Float32x3, offset: 0, name: "position".into() },
            VertexAttribute { location: 1, format: GpuFormat::Float32x3, offset: 12, name: "normal".into() },
        ];
        let _pipe = dev.create_pipeline(shader, layout, 24);
        assert_eq!(dev.pipeline_count(), 1);
        assert_eq!(dev.shader_count(), 1);
    }

    #[test]
    fn command_buffer_recording() {
        let dev = GpuDevice::default();
        let mut cmd = dev.create_command_buffer("frame");
        cmd.clear_color(0.1, 0.1, 0.1, 1.0);
        cmd.clear_depth(1.0);
        cmd.set_viewport(0, 0, 1920, 1080);
        cmd.bind_pipeline(1);
        cmd.bind_vertex_buffer(2, 0);
        cmd.bind_index_buffer(3);
        cmd.bind_texture(4, 0);
        cmd.draw_indexed(36);
        assert_eq!(cmd.command_count(), 8);
    }

    #[test]
    fn instanced_draw() {
        let mut cmd = GpuCommandBuffer::new("instanced");
        cmd.draw_instanced(36, 100);
        assert!(matches!(cmd.commands[0], GpuCommand::DrawInstanced { vertex_count: 36, instance_count: 100 }));
    }

    #[test]
    fn destroy_resources() {
        let mut dev = GpuDevice::default();
        let buf = dev.create_buffer(100, BufferUsage::Vertex);
        let tex = dev.create_texture(64, 64, TextureFormat::Rgba8);
        dev.destroy_buffer(buf);
        dev.destroy_texture(tex);
        assert_eq!(dev.buffer_count(), 0);
        assert_eq!(dev.texture_count(), 0);
    }

    #[test]
    fn memory_estimation() {
        let mut dev = GpuDevice::default();
        dev.create_buffer(1000, BufferUsage::Vertex);
        dev.create_texture(256, 256, TextureFormat::Rgba8);
        // 1000 (buffer) + 256*256*4 (texture) = 1000 + 262144
        assert_eq!(dev.estimated_memory(), 1000 + 262144);
    }

    #[test]
    fn format_byte_sizes() {
        assert_eq!(GpuFormat::Float32.byte_size(), 4);
        assert_eq!(GpuFormat::Float32x3.byte_size(), 12);
        assert_eq!(GpuFormat::Float32x4.byte_size(), 16);
        assert_eq!(GpuFormat::Uint16.byte_size(), 2);
    }
}
