//! A cross-platform graphics and compute library based on WebGPU.

mod future;
use future::GpuFutureCompletion;
pub use future::GpuFuture;

use arrayvec::ArrayVec;
use smallvec::SmallVec;

use std::ffi::CString;
use std::ops::Range;
use std::ptr;
use std::slice;
use std::thread;

pub use wgc::{
    binding_model::ShaderStage,
    command::{
        CommandBufferDescriptor,
        CommandEncoderDescriptor,
        LoadOp,
        RenderPassDepthStencilAttachmentDescriptor,
        StoreOp,
    },
    device::{
        BIND_BUFFER_ALIGNMENT,
    },
    instance::{
        AdapterInfo,
        BackendBit,
        DeviceDescriptor,
        Extensions,
        Limits,
        PowerPreference,
        RequestAdapterOptions,
    },
    pipeline::{
        BlendDescriptor,
        BlendFactor,
        BlendOperation,
        ColorStateDescriptor,
        ColorWrite,
        CullMode,
        DepthStencilStateDescriptor,
        FrontFace,
        IndexFormat,
        InputStepMode,
        PrimitiveTopology,
        RasterizationStateDescriptor,
        ShaderLocation,
        ShaderModuleDescriptor,
        StencilOperation,
        StencilStateFaceDescriptor,
        VertexAttributeDescriptor,
        VertexFormat,
    },
    read_spirv,
    resource::{
        AddressMode,
        BufferDescriptor,
        BufferMapAsyncStatus,
        BufferUsage,
        CompareFunction,
        FilterMode,
        SamplerDescriptor,
        TextureAspect,
        TextureDescriptor,
        TextureDimension,
        TextureFormat,
        TextureUsage,
        TextureViewDescriptor,
        TextureViewDimension,
    },
    swap_chain::{PresentMode, SwapChainDescriptor},
    BufferAddress,
    Color,
    Extent3d,
    Origin3d,
};

//TODO: avoid heap allocating vectors during resource creation.
#[derive(Default, Debug)]
struct Temp {
    //bind_group_descriptors: Vec<wgn::BindGroupDescriptor>,
//vertex_buffers: Vec<wgn::VertexBufferDescriptor>,
}

/// A handle to a physical graphics and/or compute device.
///
/// An `Adapter` can be used to open a connection to the corresponding device on the host system,
/// yielding a [`Device`] object.
#[derive(Debug)]
pub struct Adapter {
    id: wgc::id::AdapterId,
}

/// An open connection to a graphics and/or compute device.
///
/// The `Device` is the responsible for the creation of most rendering and compute resources, as
/// well as exposing [`Queue`] objects.
#[derive(Debug)]
pub struct Device {
    id: wgc::id::DeviceId,
    temp: Temp,
}

/// A handle to a GPU-accessible buffer.
#[derive(Debug)]
pub struct Buffer {
    id: wgc::id::BufferId,
    device_id: wgc::id::DeviceId,
}

/// A handle to a texture on the GPU.
#[derive(Debug)]
pub struct Texture {
    id: wgc::id::TextureId,
    owned: bool,
}

/// A handle to a texture view.
///
/// A `TextureView` object describes a texture and associated metadata needed by a
/// [`RenderPipeline`] or [`BindGroup`].
#[derive(Debug)]
pub struct TextureView {
    id: wgc::id::TextureViewId,
    owned: bool,
}

/// A handle to a sampler.
///
/// A `Sampler` object defines how a pipeline will sample from a [`TextureView`]. Samplers define
/// image filters (including anisotropy) and address (wrapping) modes, among other things. See
/// the documentation for [`SamplerDescriptor`] for more information.
#[derive(Debug)]
pub struct Sampler {
    id: wgc::id::SamplerId,
}

/// A handle to a presentable surface.
///
/// A `Surface` represents a platform-specific surface (e.g. a window) to which rendered images may
/// be presented. A `Surface` may be created with [`Surface::create`].
#[derive(Debug)]
pub struct Surface {
    id: wgc::id::SurfaceId,
}

/// A handle to a swap chain.
///
/// A `SwapChain` represents the image or series of images that will be presented to a [`Surface`].
/// A `SwapChain` may be created with [`Device::create_swap_chain`].
#[derive(Debug)]
pub struct SwapChain {
    id: wgc::id::SwapChainId,
}

/// An opaque handle to a binding group layout.
///
/// A `BindGroupLayout` is a handle to the GPU-side layout of a binding group. It can be used to
/// create a [`BindGroupDescriptor`] object, which in turn can be used to create a [`BindGroup`]
/// object with [`Device::create_bind_group`]. A series of `BindGroupLayout`s can also be used to
/// create a [`PipelineLayoutDescriptor`], which can be used to create a [`PipelineLayout`].
#[derive(Debug)]
pub struct BindGroupLayout {
    id: wgc::id::BindGroupLayoutId,
}

/// An opaque handle to a binding group.
///
/// A `BindGroup` represents the set of resources bound to the bindings described by a
/// [`BindGroupLayout`]. It can be created with [`Device::create_bind_group`]. A `BindGroup` can
/// be bound to a particular [`RenderPass`] with [`RenderPass::set_bind_group`], or to a
/// [`ComputePass`] with [`ComputePass::set_bind_group`].
#[derive(Debug)]
pub struct BindGroup {
    id: wgc::id::BindGroupId,
}

impl Drop for BindGroup {
    fn drop(&mut self) {
        wgn::wgpu_bind_group_destroy(self.id);
    }
}

/// A handle to a compiled shader module.
///
/// A `ShaderModule` represents a compiled shader module on the GPU. It can be created by passing
/// valid SPIR-V source code to [`Device::create_shader_module`]. Shader modules are used to define
/// programmable stages of a pipeline.
#[derive(Debug)]
pub struct ShaderModule {
    id: wgc::id::ShaderModuleId,
}

/// An opaque handle to a pipeline layout.
///
/// A `PipelineLayout` object describes the available binding groups of a pipeline.
#[derive(Debug)]
pub struct PipelineLayout {
    id: wgc::id::PipelineLayoutId,
}

/// A handle to a rendering (graphics) pipeline.
///
/// A `RenderPipeline` object represents a graphics pipeline and its stages, bindings, vertex
/// buffers and targets. A `RenderPipeline` may be created with [`Device::create_render_pipeline`].
#[derive(Debug)]
pub struct RenderPipeline {
    id: wgc::id::RenderPipelineId,
}

/// A handle to a compute pipeline.
#[derive(Debug)]
pub struct ComputePipeline {
    id: wgc::id::ComputePipelineId,
}

/// An opaque handle to a command buffer on the GPU.
///
/// A `CommandBuffer` represents a complete sequence of commands that may be submitted to a command
/// queue with [`Queue::submit`]. A `CommandBuffer` is obtained by recording a series of commands to
/// a [`CommandEncoder`] and then calling [`CommandEncoder::finish`].
#[derive(Debug)]
pub struct CommandBuffer {
    id: wgc::id::CommandBufferId,
}

/// An object that encodes GPU operations.
///
/// A `CommandEncoder` can record [`RenderPass`]es, [`ComputePass`]es, and transfer operations
/// between driver-managed resources like [`Buffer`]s and [`Texture`]s.
///
/// When finished recording, call [`CommandEncoder::finish`] to obtain a [`CommandBuffer`] which may
/// be submitted for execution.
#[derive(Debug)]
pub struct CommandEncoder {
    id: wgc::id::CommandEncoderId,
    /// This type should be !Send !Sync, because it represents an allocation on this thread's
    /// command buffer.
    _p: std::marker::PhantomData<*const u8>,
}

/// An in-progress recording of a render pass.
#[derive(Debug)]
pub struct RenderPass {
    id: wgc::id::RenderPassId,
}

/// An in-progress recording of a compute pass.
#[derive(Debug)]
pub struct ComputePass {
    id: wgc::id::ComputePassId,
}

/// A handle to a command queue on a device.
///
/// A `Queue` executes recorded [`CommandBuffer`] objects.
#[derive(Debug)]
pub struct Queue {
    id: wgc::id::QueueId,
}

/// A resource that can be bound to a pipeline.
#[derive(Clone, Debug)]
pub enum BindingResource<'a> {
    Buffer {
        buffer: &'a Buffer,
        range: Range<BufferAddress>,
    },
    Sampler(&'a Sampler),
    TextureView(&'a TextureView),
}

/// A bindable resource and the slot to bind it to.
#[derive(Clone, Debug)]
pub struct Binding<'a> {
    pub binding: u32,
    pub resource: BindingResource<'a>,
}

/// Specific type of a binding..
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum BindingType {
    UniformBuffer {
        dynamic: bool,
    },
    StorageBuffer {
        dynamic: bool,
        readonly: bool,
    },
    Sampler,
    SampledTexture {
        multisampled: bool,
        dimension: TextureViewDimension,
    },
    StorageTexture {
        dimension: TextureViewDimension,
    },
}

/// A description of a single binding inside a bind group.
#[derive(Clone, Debug, Hash)]
pub struct BindGroupLayoutBinding {
    pub binding: u32,
    pub visibility: ShaderStage,
    pub ty: BindingType,
}

#[derive(Clone, Debug)]
pub struct BindGroupLayoutDescriptor<'a> {
    pub bindings: &'a [BindGroupLayoutBinding],
}

/// A description of a group of bindings and the resources to be bound.
#[derive(Clone, Debug)]
pub struct BindGroupDescriptor<'a> {
    /// The layout for this bind group.
    pub layout: &'a BindGroupLayout,

    /// The resources to bind to this bind group.
    pub bindings: &'a [Binding<'a>],
}

/// A description of a pipeline layout.
///
/// A `PipelineLayoutDescriptor` can be passed to [`Device::create_pipeline_layout`] to obtain a
/// [`PipelineLayout`].
#[derive(Clone, Debug)]
pub struct PipelineLayoutDescriptor<'a> {
    pub bind_group_layouts: &'a [&'a BindGroupLayout],
}

/// A description of a programmable pipeline stage.
#[derive(Clone, Debug)]
pub struct ProgrammableStageDescriptor<'a> {
    /// The compiled shader module for this stage.
    pub module: &'a ShaderModule,

    /// The name of the entry point in the compiled shader.
    pub entry_point: &'a str,
}

/// A description of a vertex buffer.
#[derive(Clone, Debug)]
pub struct VertexBufferDescriptor<'a> {
    /// The stride, in bytes, between elements of this buffer.
    pub stride: BufferAddress,

    pub step_mode: InputStepMode,

    /// The list of attributes which comprise a single vertex.
    pub attributes: &'a [VertexAttributeDescriptor],
}

/// A complete description of a render (graphics) pipeline.
#[derive(Clone, Debug)]
pub struct RenderPipelineDescriptor<'a> {
    /// The layout of bind groups for this pipeline.
    pub layout: &'a PipelineLayout,

    /// The compiled vertex stage and its entry point.
    pub vertex_stage: ProgrammableStageDescriptor<'a>,

    /// The compiled fragment stage and its entry point, if any.
    pub fragment_stage: Option<ProgrammableStageDescriptor<'a>>,

    /// The rasterization process for this pipeline.
    pub rasterization_state: Option<RasterizationStateDescriptor>,

    /// The primitive topology used to interpret vertices.
    pub primitive_topology: PrimitiveTopology,

    /// The effect of draw calls on the color aspect of the output target.
    pub color_states: &'a [ColorStateDescriptor],

    /// The effect of draw calls on the depth and stencil aspects of the output target, if any.
    pub depth_stencil_state: Option<DepthStencilStateDescriptor>,

    /// The format of any index buffers used with this pipeline.
    pub index_format: IndexFormat,

    /// The format of any vertex buffers used with this pipeline.
    pub vertex_buffers: &'a [VertexBufferDescriptor<'a>],

    /// The number of samples calculated per pixel (for MSAA).
    pub sample_count: u32,

    /// Bitmask that restricts the samples of a pixel modified by this pipeline.
    pub sample_mask: u32,

    /// When enabled, produces another sample mask per pixel based on the alpha output value, that
    /// is ANDed with the sample_mask and the primitive coverage to restrict the set of samples
    /// affected by a primitive.
    /// The implicit mask produced for alpha of zero is guaranteed to be zero, and for alpha of one
    /// is guaranteed to be all 1-s.
    pub alpha_to_coverage_enabled: bool,
}

/// A complete description of a compute pipeline.
#[derive(Clone, Debug)]
pub struct ComputePipelineDescriptor<'a> {
    /// The layout of bind groups for this pipeline.
    pub layout: &'a PipelineLayout,

    /// The compiled compute stage and its entry point.
    pub compute_stage: ProgrammableStageDescriptor<'a>,
}

/// A description of all the attachments of a render pass.
#[derive(Debug)]
pub struct RenderPassDescriptor<'a> {
    /// The color attachments of the render pass.
    pub color_attachments: &'a [RenderPassColorAttachmentDescriptor<'a>],

    /// The depth and stencil attachment of the render pass, if any.
    pub depth_stencil_attachment:
        Option<RenderPassDepthStencilAttachmentDescriptor<&'a TextureView>>,
}

/// A description of a color attachment.
#[derive(Clone, Debug)]
pub struct RenderPassColorAttachmentDescriptor<'a> {
    /// The actual color attachment.
    pub attachment: &'a TextureView,

    /// The resolve target for this color attachment, if any.
    pub resolve_target: Option<&'a TextureView>,

    /// The beginning-of-pass load operation for this color attachment.
    pub load_op: LoadOp,

    /// The end-of-pass store operation for this color attachment.
    pub store_op: StoreOp,

    /// The color that will be assigned to every pixel of this attachment when cleared.
    pub clear_color: Color,
}

/// A swap chain image that can be rendered to.
#[derive(Debug)]
pub struct SwapChainOutput<'a> {
    pub view: TextureView,
    swap_chain_id: &'a wgc::id::SwapChainId,
}

/// A view of a buffer which can be used to copy to or from a texture.
#[derive(Clone, Debug)]
pub struct BufferCopyView<'a> {
    /// The buffer to be copied to or from.
    pub buffer: &'a Buffer,

    /// The offset in bytes from the start of the buffer. This must be aligned to 512 bytes.
    pub offset: BufferAddress,

    /// The size in bytes of a single row of the texture. This must be a multiple of 256 bytes.
    pub row_pitch: u32,

    /// The height in texels of the imaginary texture view overlaid on the buffer.
    pub image_height: u32,
}

impl<'a> BufferCopyView<'a> {
    fn into_native(self) -> wgc::command::BufferCopyView {
        wgc::command::BufferCopyView {
            buffer: self.buffer.id,
            offset: self.offset,
            row_pitch: self.row_pitch,
            image_height: self.image_height,
        }
    }
}

/// A view of a texture which can be used to copy to or from a buffer or another texture.
#[derive(Clone, Debug)]
pub struct TextureCopyView<'a> {
    /// The texture to be copied to or from.
    pub texture: &'a Texture,

    /// The target mip level of the texture.
    pub mip_level: u32,

    /// The target layer of the texture.
    pub array_layer: u32,

    /// The base texel of the texture in the selected `mip_level`.
    pub origin: Origin3d,
}

impl<'a> TextureCopyView<'a> {
    fn into_native(self) -> wgc::command::TextureCopyView {
        wgc::command::TextureCopyView {
            texture: self.texture.id,
            mip_level: self.mip_level,
            array_layer: self.array_layer,
            origin: self.origin,
        }
    }
}

/// A buffer being created, mapped in host memory.
pub struct CreateBufferMapped<'a> {
    id: wgc::id::BufferId,
    pub data: &'a mut [u8],
    device_id: wgc::id::DeviceId,
}

impl CreateBufferMapped<'_> {
    /// Unmaps the buffer from host memory and returns a [`Buffer`].
    pub fn finish(self) -> Buffer {
        wgn::wgpu_buffer_unmap(self.id);
        Buffer { device_id: self.device_id, id: self.id }
    }
}

impl Surface {
    /// Creates a surface from a raw window handle.
    pub fn create<W: raw_window_handle::HasRawWindowHandle>(window: &W) -> Self {
        Surface {
            id: wgn::wgpu_create_surface(window.raw_window_handle()),
        }
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub fn create_surface_from_core_animation_layer(layer: *mut std::ffi::c_void) -> Self {
        Surface {
            id: wgn::wgpu_create_surface_from_metal_layer(layer),
        }
    }
}

impl Adapter {
    /// Retrieves an [`Adapter`] which matches the given options.
    ///
    /// Some options are "soft", so treated as non-mandatory. Others are "hard".
    ///
    /// If no adapters are found that suffice all the "hard" options, `None` is returned.
    pub fn request(options: &RequestAdapterOptions, backends: BackendBit) -> Option<Self> {
        unsafe extern "C" fn adapter_callback(
            id: wgc::id::AdapterId,
            user_data: *mut std::ffi::c_void,
        ) {
            *(user_data as *mut wgc::id::AdapterId) = id;
        }

        let mut id = wgc::id::AdapterId::ERROR;
        wgn::wgpu_request_adapter_async(
            Some(options),
            backends,
            adapter_callback,
            &mut id as *mut _ as *mut std::ffi::c_void,
        );
        Some(Adapter { id })
    }

    /// Requests a connection to a physical device, creating a logical device.
    /// Returns the device together with a queue that executes command buffers.
    ///
    /// # Panics
    ///
    /// Panics if the extensions specified by `desc` are not supported by this adapter.
    pub fn request_device(&self, desc: &DeviceDescriptor) -> (Device, Queue) {
        let device = Device {
            id: wgn::wgpu_adapter_request_device(self.id, Some(desc)),
            temp: Temp::default(),
        };
        let queue = Queue {
            id: wgn::wgpu_device_get_queue(device.id),
        };
        (device, queue)
    }

    pub fn get_info(&self) -> AdapterInfo {
        wgn::wgpu_adapter_get_info(self.id)
    }
}

impl Device {
    /// Check for resource cleanups and mapping callbacks.
    pub fn poll(&self, force_wait: bool) {
        wgn::wgpu_device_poll(self.id, force_wait);
    }

    /// Creates a shader module from SPIR-V source code.
    pub fn create_shader_module(&self, spv: &[u32]) -> ShaderModule {
        let desc = wgc::pipeline::ShaderModuleDescriptor {
            code: wgc::U32Array {
                bytes: spv.as_ptr(),
                length: spv.len(),
            },
        };
        ShaderModule {
            id: wgn::wgpu_device_create_shader_module(self.id, &desc),
        }
    }

    /// Creates an empty [`CommandEncoder`].
    pub fn create_command_encoder(&self, desc: &CommandEncoderDescriptor) -> CommandEncoder {
        CommandEncoder {
            id: wgn::wgpu_device_create_command_encoder(self.id, Some(desc)),
            _p: Default::default(),
        }
    }

    /// Creates a new bind group.
    pub fn create_bind_group(&self, desc: &BindGroupDescriptor) -> BindGroup {
        use wgc::binding_model as bm;

        let bindings = desc
            .bindings
            .iter()
            .map(|binding| bm::BindGroupBinding {
                binding: binding.binding,
                resource: match binding.resource {
                    BindingResource::Buffer {
                        ref buffer,
                        ref range,
                    } => bm::BindingResource::Buffer(bm::BufferBinding {
                        buffer: buffer.id,
                        offset: range.start,
                        size: range.end - range.start,
                    }),
                    BindingResource::Sampler(ref sampler) => {
                        bm::BindingResource::Sampler(sampler.id)
                    }
                    BindingResource::TextureView(ref texture_view) => {
                        bm::BindingResource::TextureView(texture_view.id)
                    }
                },
            })
            .collect::<Vec<_>>();

        BindGroup {
            id: wgn::wgpu_device_create_bind_group(
                self.id,
                &bm::BindGroupDescriptor {
                    layout: desc.layout.id,
                    bindings: bindings.as_ptr(),
                    bindings_length: bindings.len(),
                },
            ),
        }
    }

    /// Creates a bind group layout.
    pub fn create_bind_group_layout(&self, desc: &BindGroupLayoutDescriptor) -> BindGroupLayout {
        use wgc::binding_model as bm;

        let temp_layouts = desc
            .bindings
            .iter()
            .map(|bind| bm::BindGroupLayoutBinding {
                binding: bind.binding,
                visibility: bind.visibility,
                ty: match bind.ty {
                    BindingType::UniformBuffer { .. } => bm::BindingType::UniformBuffer,
                    BindingType::StorageBuffer {
                        readonly: false, ..
                    } => bm::BindingType::StorageBuffer,
                    BindingType::StorageBuffer { readonly: true, .. } => {
                        bm::BindingType::ReadonlyStorageBuffer
                    }
                    BindingType::Sampler => bm::BindingType::Sampler,
                    BindingType::SampledTexture { .. } => bm::BindingType::SampledTexture,
                    BindingType::StorageTexture { .. } => bm::BindingType::StorageTexture,
                },
                dynamic: match bind.ty {
                    BindingType::UniformBuffer { dynamic }
                    | BindingType::StorageBuffer { dynamic, .. } => dynamic,
                    _ => false,
                },
                multisampled: match bind.ty {
                    BindingType::SampledTexture { multisampled, .. } => multisampled,
                    _ => false,
                },
                texture_dimension: match bind.ty {
                    BindingType::SampledTexture { dimension, .. }
                    | BindingType::StorageTexture { dimension } => dimension,
                    _ => TextureViewDimension::D2,
                },
            })
            .collect::<Vec<_>>();
        BindGroupLayout {
            id: wgn::wgpu_device_create_bind_group_layout(
                self.id,
                &bm::BindGroupLayoutDescriptor {
                    bindings: temp_layouts.as_ptr(),
                    bindings_length: temp_layouts.len(),
                },
            ),
        }
    }

    /// Creates a pipeline layout.
    pub fn create_pipeline_layout(&self, desc: &PipelineLayoutDescriptor) -> PipelineLayout {
        //TODO: avoid allocation here
        let temp_layouts = desc
            .bind_group_layouts
            .iter()
            .map(|bgl| bgl.id)
            .collect::<Vec<_>>();
        PipelineLayout {
            id: wgn::wgpu_device_create_pipeline_layout(
                self.id,
                &wgc::binding_model::PipelineLayoutDescriptor {
                    bind_group_layouts: temp_layouts.as_ptr(),
                    bind_group_layouts_length: temp_layouts.len(),
                },
            ),
        }
    }

    /// Creates a render pipeline.
    pub fn create_render_pipeline(&self, desc: &RenderPipelineDescriptor) -> RenderPipeline {
        use wgc::pipeline as pipe;

        let vertex_entry_point = CString::new(desc.vertex_stage.entry_point).unwrap();
        let vertex_stage = pipe::ProgrammableStageDescriptor {
            module: desc.vertex_stage.module.id,
            entry_point: vertex_entry_point.as_ptr(),
        };
        let (_fragment_entry_point, fragment_stage) =
            if let Some(fragment_stage) = &desc.fragment_stage {
                let fragment_entry_point = CString::new(fragment_stage.entry_point).unwrap();
                let fragment_stage = pipe::ProgrammableStageDescriptor {
                    module: fragment_stage.module.id,
                    entry_point: fragment_entry_point.as_ptr(),
                };
                (fragment_entry_point, Some(fragment_stage))
            } else {
                (CString::default(), None)
            };

        let temp_color_states = desc.color_states.to_vec();
        let temp_vertex_buffers = desc
            .vertex_buffers
            .iter()
            .map(|vbuf| pipe::VertexBufferDescriptor {
                stride: vbuf.stride,
                step_mode: vbuf.step_mode,
                attributes: vbuf.attributes.as_ptr(),
                attributes_length: vbuf.attributes.len(),
            })
            .collect::<Vec<_>>();

        RenderPipeline {
            id: wgn::wgpu_device_create_render_pipeline(
                self.id,
                &pipe::RenderPipelineDescriptor {
                    layout: desc.layout.id,
                    vertex_stage,
                    fragment_stage: fragment_stage
                        .as_ref()
                        .map_or(ptr::null(), |fs| fs as *const _),
                    rasterization_state: desc
                        .rasterization_state
                        .as_ref()
                        .map_or(ptr::null(), |p| p as *const _),
                    primitive_topology: desc.primitive_topology,
                    color_states: temp_color_states.as_ptr(),
                    color_states_length: temp_color_states.len(),
                    depth_stencil_state: desc
                        .depth_stencil_state
                        .as_ref()
                        .map_or(ptr::null(), |p| p as *const _),
                    vertex_input: pipe::VertexInputDescriptor {
                        index_format: desc.index_format,
                        vertex_buffers: temp_vertex_buffers.as_ptr(),
                        vertex_buffers_length: temp_vertex_buffers.len(),
                    },
                    sample_count: desc.sample_count,
                    sample_mask: desc.sample_mask,
                    alpha_to_coverage_enabled: desc.alpha_to_coverage_enabled,
                },
            ),
        }
    }

    /// Creates a compute pipeline.
    pub fn create_compute_pipeline(&self, desc: &ComputePipelineDescriptor) -> ComputePipeline {
        use wgc::pipeline as pipe;

        let entry_point = CString::new(desc.compute_stage.entry_point).unwrap();

        ComputePipeline {
            id: wgn::wgpu_device_create_compute_pipeline(
                self.id,
                &pipe::ComputePipelineDescriptor {
                    layout: desc.layout.id,
                    compute_stage: pipe::ProgrammableStageDescriptor {
                        module: desc.compute_stage.module.id,
                        entry_point: entry_point.as_ptr(),
                    },
                },
            ),
        }
    }

    /// Creates a new buffer.
    pub fn create_buffer(&self, desc: &BufferDescriptor) -> Buffer {
        Buffer {
            device_id: self.id,
            id: wgn::wgpu_device_create_buffer(self.id, desc),
        }
    }

    /// Creates a new buffer and maps it into host-visible memory.
    ///
    /// This returns a [`CreateBufferMapped`], which exposes a `&mut [u8]`. The actual [`Buffer`]
    /// will not be created until calling [`CreateBufferMapped::finish`].
    pub fn create_buffer_mapped(&self, size: usize, usage: BufferUsage) -> CreateBufferMapped<'_> {
        assert_ne!(size, 0);

        let desc = BufferDescriptor {
            size: size as BufferAddress,
            usage,
        };
        let mut ptr: *mut u8 = std::ptr::null_mut();

        let id = wgn::wgpu_device_create_buffer_mapped(self.id, &desc, &mut ptr as *mut *mut u8);

        let data = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, size) };

        CreateBufferMapped { device_id: self.id, id, data }
    }

    /// Creates a new buffer, maps it into host-visible memory, copies data from the given slice,
    /// and finally unmaps it, returning a [`Buffer`].
    pub fn create_buffer_with_data(&self, data: &[u8], usage: BufferUsage) -> Buffer {
        let mapped = self.create_buffer_mapped(data.len(), usage);
        mapped.data.copy_from_slice(data);
        mapped.finish()
    }

    /// Creates a new [`Texture`].
    ///
    /// `desc` specifies the general format of the texture.
    pub fn create_texture(&self, desc: &TextureDescriptor) -> Texture {
        Texture {
            id: wgn::wgpu_device_create_texture(self.id, desc),
            owned: true,
        }
    }

    /// Creates a new [`Sampler`].
    ///
    /// `desc` specifies the behavior of the sampler.
    pub fn create_sampler(&self, desc: &SamplerDescriptor) -> Sampler {
        Sampler {
            id: wgn::wgpu_device_create_sampler(self.id, desc),
        }
    }

    /// Create a new [`SwapChain`] which targets `surface`.
    pub fn create_swap_chain(&self, surface: &Surface, desc: &SwapChainDescriptor) -> SwapChain {
        SwapChain {
            id: wgn::wgpu_device_create_swap_chain(self.id, surface.id, desc),
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        wgn::wgpu_device_poll(self.id, true);
        //TODO: make this work in general
        #[cfg(feature = "metal-auto-capture")]
        wgn::wgpu_device_destroy(self.id);
    }
}

pub struct BufferReadMapping {
    data: *const u8,
    size: usize,
    buffer_id: wgc::id::BufferId,
}
//TODO: proper error type
pub type BufferMapReadResult = Result<BufferReadMapping, ()>;

impl BufferReadMapping
{
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.data as *const u8, self.size)
        }
    }
}

impl Drop for BufferReadMapping {
    fn drop(&mut self) {
        wgn::wgpu_buffer_unmap(self.buffer_id);
    }
}

pub struct BufferWriteMapping {
    data: *mut u8,
    size: usize,
    buffer_id: wgc::id::BufferId,
}
//TODO: proper error type
pub type BufferMapWriteResult = Result<BufferWriteMapping, ()>;

impl BufferWriteMapping
{
    pub fn as_slice(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.data as *mut u8, self.size)
        }
    }
}

impl Drop for BufferWriteMapping {
    fn drop(&mut self) {
        wgn::wgpu_buffer_unmap(self.buffer_id);
    }
}

pub struct BufferAsyncMapping<T> {
    pub data: T,
    buffer_id: wgc::id::BufferId,
}
//TODO: proper error type
pub type BufferMapAsyncResult<T> = Result<BufferAsyncMapping<T>, ()>;

impl<T> Drop for BufferAsyncMapping<T> {
    fn drop(&mut self) {
        wgn::wgpu_buffer_unmap(self.buffer_id);
    }
}

struct BufferMapReadFutureUserData
{
    size: BufferAddress,
    completion: GpuFutureCompletion<BufferMapReadResult>,
    buffer_id: wgc::id::BufferId,
}

struct BufferMapWriteFutureUserData
{
    size: BufferAddress,
    completion: GpuFutureCompletion<BufferMapWriteResult>,
    buffer_id: wgc::id::BufferId,
}

impl Buffer {
    /// Map the buffer for reading. The result is returned in a future.
    pub fn map_read(&self, start: BufferAddress, size: BufferAddress) -> GpuFuture<BufferMapReadResult>
    {
        let (future, completion) = future::new_gpu_future(self.device_id);

        extern "C" fn buffer_map_read_future_wrapper(
            status: wgc::resource::BufferMapAsyncStatus,
            data: *const u8,
            user_data: *mut u8,
        )
        {
            let user_data =
                unsafe { Box::from_raw(user_data as *mut BufferMapReadFutureUserData) };
            if let wgc::resource::BufferMapAsyncStatus::Success = status {
                user_data.completion.complete(Ok(BufferReadMapping {
                    data,
                    size: user_data.size as usize,
                    buffer_id: user_data.buffer_id,
                }));
            } else {
                user_data.completion.complete(Err(()));
            }
        }

        let user_data = Box::new(BufferMapReadFutureUserData {
            size,
            completion,
            buffer_id: self.id,
        });
        wgn::wgpu_buffer_map_read_async(
            self.id,
            start,
            size,
            buffer_map_read_future_wrapper,
            Box::into_raw(user_data) as *mut u8,
        );

        future
    }

    /// Map the buffer for writing. The result is returned in a future.
    pub fn map_write(&self, start: BufferAddress, size: BufferAddress) -> GpuFuture<BufferMapWriteResult>
    {
        let (future, completion) = future::new_gpu_future(self.device_id);

        extern "C" fn buffer_map_write_future_wrapper(
            status: wgc::resource::BufferMapAsyncStatus,
            data: *mut u8,
            user_data: *mut u8,
        )
        {
            let user_data =
                unsafe { Box::from_raw(user_data as *mut BufferMapWriteFutureUserData) };
            if let wgc::resource::BufferMapAsyncStatus::Success = status {
                user_data.completion.complete(Ok(BufferWriteMapping {
                    data,
                    size: user_data.size as usize,
                    buffer_id: user_data.buffer_id,
                }));
            } else {
                user_data.completion.complete(Err(()));
            }
        }

        let user_data = Box::new(BufferMapWriteFutureUserData {
            size,
            completion,
            buffer_id: self.id,
        });
        wgn::wgpu_buffer_map_write_async(
            self.id,
            start,
            size,
            buffer_map_write_future_wrapper,
            Box::into_raw(user_data) as *mut u8,
        );

        future
    }

    /// Flushes any pending write operations and unmaps the buffer from host memory.
    pub fn unmap(&self) {
        wgn::wgpu_buffer_unmap(self.id);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        wgn::wgpu_buffer_destroy(self.id);
    }
}

impl Texture {
    /// Creates a view of this texture.
    pub fn create_view(&self, desc: &TextureViewDescriptor) -> TextureView {
        TextureView {
            id: wgn::wgpu_texture_create_view(self.id, Some(desc)),
            owned: true,
        }
    }

    /// Creates a default view of this whole texture.
    pub fn create_default_view(&self) -> TextureView {
        TextureView {
            id: wgn::wgpu_texture_create_view(self.id, None),
            owned: true,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.owned {
            wgn::wgpu_texture_destroy(self.id);
        }
    }
}

impl Drop for TextureView {
    fn drop(&mut self) {
        if self.owned {
            wgn::wgpu_texture_view_destroy(self.id);
        }
    }
}

impl CommandEncoder {
    /// Finishes recording and returns a [`CommandBuffer`] that can be submitted for execution.
    pub fn finish(self) -> CommandBuffer {
        CommandBuffer {
            id: wgn::wgpu_command_encoder_finish(self.id, None),
        }
    }

    /// Begins recording of a render pass.
    ///
    /// This function returns a [`RenderPass`] object which records a single render pass.
    pub fn begin_render_pass(&mut self, desc: &RenderPassDescriptor) -> RenderPass {
        let colors = desc
            .color_attachments
            .iter()
            .map(|ca| wgc::command::RenderPassColorAttachmentDescriptor {
                attachment: ca.attachment.id,
                resolve_target: ca.resolve_target.map_or(ptr::null(), |v| &v.id as *const _),
                load_op: ca.load_op,
                store_op: ca.store_op,
                clear_color: ca.clear_color,
            })
            .collect::<ArrayVec<[_; 4]>>();

        let depth_stencil = desc.depth_stencil_attachment.as_ref().map(|dsa| {
            RenderPassDepthStencilAttachmentDescriptor {
                attachment: dsa.attachment.id,
                depth_load_op: dsa.depth_load_op,
                depth_store_op: dsa.depth_store_op,
                clear_depth: dsa.clear_depth,
                stencil_load_op: dsa.stencil_load_op,
                stencil_store_op: dsa.stencil_store_op,
                clear_stencil: dsa.clear_stencil,
            }
        });

        RenderPass {
            id: wgn::wgpu_command_encoder_begin_render_pass(
                self.id,
                &wgc::command::RenderPassDescriptor {
                    color_attachments: colors.as_ptr(),
                    color_attachments_length: colors.len(),
                    depth_stencil_attachment: depth_stencil
                        .as_ref()
                        .map(|at| at as *const _)
                        .unwrap_or(ptr::null()),
                },
            ),
        }
    }

    /// Begins recording of a compute pass.
    ///
    /// This function returns a [`ComputePass`] object which records a single compute pass.
    pub fn begin_compute_pass(&mut self) -> ComputePass {
        ComputePass {
            id: wgn::wgpu_command_encoder_begin_compute_pass(self.id, None),
        }
    }

    /// Copy data from one buffer to another.
    pub fn copy_buffer_to_buffer(
        &mut self,
        source: &Buffer,
        source_offset: BufferAddress,
        destination: &Buffer,
        destination_offset: BufferAddress,
        copy_size: BufferAddress,
    ) {
        wgn::wgpu_command_encoder_copy_buffer_to_buffer(
            self.id,
            source.id,
            source_offset,
            destination.id,
            destination_offset,
            copy_size,
        );
    }

    /// Copy data from a buffer to a texture.
    pub fn copy_buffer_to_texture(
        &mut self,
        source: BufferCopyView,
        destination: TextureCopyView,
        copy_size: Extent3d,
    ) {
        wgn::wgpu_command_encoder_copy_buffer_to_texture(
            self.id,
            &source.into_native(),
            &destination.into_native(),
            copy_size,
        );
    }

    /// Copy data from a texture to a buffer.
    pub fn copy_texture_to_buffer(
        &mut self,
        source: TextureCopyView,
        destination: BufferCopyView,
        copy_size: Extent3d,
    ) {
        wgn::wgpu_command_encoder_copy_texture_to_buffer(
            self.id,
            &source.into_native(),
            &destination.into_native(),
            copy_size,
        );
    }

    /// Copy data from one texture to another.
    pub fn copy_texture_to_texture(
        &mut self,
        source: TextureCopyView,
        destination: TextureCopyView,
        copy_size: Extent3d,
    ) {
        wgn::wgpu_command_encoder_copy_texture_to_texture(
            self.id,
            &source.into_native(),
            &destination.into_native(),
            copy_size,
        );
    }
}

impl RenderPass {
    /// Sets the active bind group for a given bind group index.
    pub fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: &BindGroup,
        offsets: &[BufferAddress],
    ) {
        wgn::wgpu_render_pass_set_bind_group(
            self.id,
            index,
            bind_group.id,
            offsets.as_ptr(),
            offsets.len(),
        );
    }

    /// Sets the active render pipeline.
    ///
    /// Subsequent draw calls will exhibit the behavior defined by `pipeline`.
    pub fn set_pipeline(&mut self, pipeline: &RenderPipeline) {
        wgn::wgpu_render_pass_set_pipeline(self.id, pipeline.id);
    }

    pub fn set_blend_color(&mut self, color: Color) {
        wgn::wgpu_render_pass_set_blend_color(self.id, &color);
    }

    /// Sets the active index buffer.
    ///
    /// Subsequent calls to [`draw_indexed`](RenderPass::draw_indexed) on this [`RenderPass`] will
    /// use `buffer` as the source index buffer.
    pub fn set_index_buffer(&mut self, buffer: &Buffer, offset: BufferAddress) {
        wgn::wgpu_render_pass_set_index_buffer(self.id, buffer.id, offset);
    }

    /// Sets the active vertex buffers, starting from `start_slot`.
    ///
    /// Each element of `buffer_pairs` describes a vertex buffer and an offset in bytes into that
    /// buffer. The offset must be aligned to a multiple of 4 bytes.
    pub fn set_vertex_buffers(
        &mut self,
        start_slot: u32,
        buffer_pairs: &[(&Buffer, BufferAddress)],
    ) {
        let mut buffers = Vec::new();
        let mut offsets = Vec::new();
        for &(buffer, offset) in buffer_pairs {
            buffers.push(buffer.id);
            offsets.push(offset);
        }
        wgn::wgpu_render_pass_set_vertex_buffers(
            self.id,
            start_slot,
            buffers.as_ptr(),
            offsets.as_ptr(),
            buffer_pairs.len(),
        );
    }

    /// Sets the scissor region.
    ///
    /// Subsequent draw calls will discard any fragments that fall outside this region.
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        wgn::wgpu_render_pass_set_scissor_rect(self.id, x, y, w, h)
    }

    /// Sets the viewport region.
    ///
    /// Subsequent draw calls will draw any fragments in this region.
    pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
        wgn::wgpu_render_pass_set_viewport(self.id, x, y, w, h, min_depth, max_depth)
    }

    /// Sets the stencil reference.
    ///
    /// Subsequent stencil tests will test against this value.
    pub fn set_stencil_reference(&mut self, reference: u32) {
        wgn::wgpu_render_pass_set_stencil_reference(self.id, reference)
    }

    /// Draws primitives from the active vertex buffer(s).
    ///
    /// The active vertex buffers can be set with [`RenderPass::set_vertex_buffers`].
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        wgn::wgpu_render_pass_draw(
            self.id,
            vertices.end - vertices.start,
            instances.end - instances.start,
            vertices.start,
            instances.start,
        );
    }

    /// Draws indexed primitives using the active index buffer and the active vertex buffers.
    ///
    /// The active index buffer can be set with [`RenderPass::set_index_buffer`], while the active
    /// vertex buffers can be set with [`RenderPass::set_vertex_buffers`].
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        wgn::wgpu_render_pass_draw_indexed(
            self.id,
            indices.end - indices.start,
            instances.end - instances.start,
            indices.start,
            base_vertex,
            instances.start,
        );
    }

    /// Draws primitives from the active vertex buffer(s) based on the contents of the `indirect_buffer`.
    ///
    /// The active vertex buffers can be set with [`RenderPass::set_vertex_buffers`].
    pub fn draw_indirect(&mut self, indirect_buffer: &Buffer, indirect_offset: BufferAddress) {
        wgn::wgpu_render_pass_draw_indirect(self.id, indirect_buffer.id, indirect_offset);
    }

    /// Draws indexed primitives using the active index buffer and the active vertex buffers,
    /// based on the contents of the `indirect_buffer`.
    ///
    /// The active index buffer can be set with [`RenderPass::set_index_buffer`], while the active
    /// vertex buffers can be set with [`RenderPass::set_vertex_buffers`].
    pub fn draw_indexed_indirect(
        &mut self,
        indirect_buffer: &Buffer,
        indirect_offset: BufferAddress,
    ) {
        wgn::wgpu_render_pass_draw_indexed_indirect(self.id, indirect_buffer.id, indirect_offset);
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        if !thread::panicking() {
            wgn::wgpu_render_pass_end_pass(self.id);
        }
    }
}

impl ComputePass {
    /// Sets the active bind group for a given bind group index.
    pub fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: &BindGroup,
        offsets: &[BufferAddress],
    ) {
        wgn::wgpu_compute_pass_set_bind_group(
            self.id,
            index,
            bind_group.id,
            offsets.as_ptr(),
            offsets.len(),
        );
    }

    /// Sets the active compute pipeline.
    pub fn set_pipeline(&mut self, pipeline: &ComputePipeline) {
        wgn::wgpu_compute_pass_set_pipeline(self.id, pipeline.id);
    }

    /// Dispatches compute work operations.
    ///
    /// `x`, `y` and `z` denote the number of work groups to dispatch in each dimension.
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        wgn::wgpu_compute_pass_dispatch(self.id, x, y, z);
    }

    /// Dispatches compute work operations, based on the contents of the `indirect_buffer`.
    pub fn dispatch_indirect(&mut self, indirect_buffer: &Buffer, indirect_offset: BufferAddress) {
        wgn::wgpu_compute_pass_dispatch_indirect(self.id, indirect_buffer.id, indirect_offset);
    }
}

impl Drop for ComputePass {
    fn drop(&mut self) {
        if !thread::panicking() {
            wgn::wgpu_compute_pass_end_pass(self.id);
        }
    }
}

impl Queue {
    /// Submits a series of finished command buffers for execution.
    pub fn submit(&self, command_buffers: &[CommandBuffer]) {
        let temp_command_buffers = command_buffers.iter()
            .map(|cb| cb.id)
            .collect::<SmallVec<[_; 4]>>();

        wgn::wgpu_queue_submit(
            self.id,
            temp_command_buffers.as_ptr(),
            command_buffers.len(),
        );
    }
}

impl<'a> Drop for SwapChainOutput<'a> {
    fn drop(&mut self) {
        if !thread::panicking() {
            wgn::wgpu_swap_chain_present(*self.swap_chain_id);
        }
    }
}

impl SwapChain {
    /// Returns the next texture to be presented by the swapchain for drawing.
    ///
    /// When the [`SwapChainOutput`] returned by this method is dropped, the swapchain will present
    /// the texture to the associated [`Surface`].
    ///
    /// Returns an `Err` if the GPU timed out when attempting to acquire the next texture.
    pub fn get_next_texture(&mut self) -> Result<SwapChainOutput, ()> {
        let output = wgn::wgpu_swap_chain_get_next_texture(self.id);
        if output.view_id == wgc::id::Id::ERROR {
            Err(())
        } else {
            Ok(SwapChainOutput {
                view: TextureView {
                    id: output.view_id,
                    owned: false,
                },
                swap_chain_id: &self.id,
            })
        }
    }
}
