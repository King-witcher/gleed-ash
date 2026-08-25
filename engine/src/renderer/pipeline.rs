//! Equivalent to modules/engine/src/pipeline.{h,cc}.

use std::io::Cursor;

use crate::device::Device;
use crate::internal_prelude::*;
use crate::mesh::Vertex;

const SHADER_SPV: &[u8] = include_bytes!("../shaders/mesh.spv");

pub(super) struct Pipeline {
    // FIELD ORDER IS DESTRUCTION ORDER: the pipeline dies before the layouts
    // that describe it. Each of them holds the device by refcount, so there is
    // no `impl Drop` here anymore — nor the leak that existed when pipeline
    // creation failed after the layouts.
    pipeline: vk::raii::Pipeline,
    layout: vk::raii::PipelineLayout,
    descriptor_set_layout: vk::raii::DescriptorSetLayout,
}

impl Pipeline {
    pub(super) fn new(device: Device, image_format: vk::Format) -> Result<Self> {
        let descriptor_set_layout = make_descriptor_set_layout(&device)?;
        let layout = make_pipeline_layout(&device, &descriptor_set_layout)?;
        let pipeline = make_pipeline(&device, &layout, image_format)?;

        Ok(Self {
            pipeline,
            layout,
            descriptor_set_layout,
        })
    }

    /// Must be bound inside a render pass.
    pub(super) fn vk_pipeline(&self) -> vk::Pipeline {
        self.pipeline.handle()
    }

    pub(super) fn layout(&self) -> vk::PipelineLayout {
        self.layout.handle()
    }

    pub(super) fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout.handle()
    }
}

fn shader_code() -> Vec<u32> {
    // read_spv validates the magic number and settles the u32 alignment —
    // what in C++ was a reinterpret_cast<const u32*> hoping for the best. It
    // is still needed even with the bytes embedded: `include_bytes!` gives a
    // 1-aligned `&[u8]`, and vkCreateShaderModule wants `[u32]`. The copy
    // happens once, at pipeline creation.
    //
    // `expect`, not `?`: these bytes are compile-time constants. If they are
    // not valid SPIR-V, the build produced a broken .spv — our bug, not an
    // environment condition the caller could handle.
    vk::util::read_spv(&mut Cursor::new(SHADER_SPV)).expect("o SPIR-V embutido é inválido")
}

fn make_descriptor_set_layout(device: &Device) -> Result<vk::raii::DescriptorSetLayout> {
    let bindings = [vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        // we can have an array of uniform buffers
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .stage_flags(vk::ShaderStageFlags::VERTEX)];

    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

    unsafe { device.vk_device().create_descriptor_set_layout(&info) }
        .context("create the descriptor set layout")
}

fn make_pipeline_layout(
    device: &Device,
    descriptor_set_layout: &vk::raii::DescriptorSetLayout,
) -> Result<vk::raii::PipelineLayout> {
    // Attaches the UBO's descriptor set layout so the shaders can read
    // `set = 0, binding = 0`.
    let set_layouts = [descriptor_set_layout.handle()];
    let info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);

    unsafe { device.vk_device().create_pipeline_layout(&info) }
        .context("create the pipeline layout")
}

fn make_pipeline(
    device: &Device,
    layout: &vk::raii::PipelineLayout,
    format: vk::Format,
) -> Result<vk::raii::Pipeline> {
    let code = shader_code();
    let shader_module = unsafe {
        device
            .vk_device()
            .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&code))
    }
    .context("create the shader module")?;

    let binding_descriptions = [Vertex::binding_description()];
    let attribute_descriptions = Vertex::attribute_descriptions();

    // Fixed functions
    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    // Viewport and scissor are dynamic (set on the command buffer).
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::BACK)
        // CCW: the projection flips Y (proj[1][1] *= -1) to match Vulkan's
        // clip space, which flips the winding the rasterizer sees. Front faces
        // become CCW.
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
        .depth_bias_slope_factor(1.0)
        .line_width(1.0);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .sample_shading_enable(false);

    let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(false)
        .color_write_mask(vk::ColorComponentFlags::RGBA)];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(&color_blend_attachments);

    // Required for dynamic rendering
    let color_attachment_formats = [format];
    let mut pipeline_rendering_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_attachment_formats);

    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(shader_module.handle())
            .name(c"vertMain"),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(shader_module.handle())
            .name(c"fragMain"),
    ];

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .push_next(&mut pipeline_rendering_info)
        .stages(&stages)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout.handle());

    // No cache for now. The chained structs above stay alive until here, and
    // the shader module goes away on its own at the end of the function —
    // including on the error path.
    unsafe {
        device
            .vk_device()
            .create_graphics_pipeline(vk::PipelineCache::null(), pipeline_info)
    }
    .context("create the graphics pipeline")
}
