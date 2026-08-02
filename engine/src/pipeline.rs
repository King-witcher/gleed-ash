//! Equivalente a modules/engine/src/pipeline.{h,cc}.

use std::io::Cursor;

use ash::vk;

use crate::device::Device;
use crate::prelude::*;
use crate::vertex::Vertex;

/// O SPIR-V vai embutido no executável. O build.rs compila `shaders/mesh.slang`
/// antes de cada build, e o `include_bytes!` resolve o caminho em relação a
/// ESTE arquivo — então um caminho errado vira erro de compilação, não uma
/// falha em runtime, e o binário não depende mais de nada no disco. O C++ lia o
/// .spv na hora e por isso dependia do CWD ser a raiz do projeto.
const SHADER_SPV: &[u8] = include_bytes!("shaders/mesh.spv");

pub struct Pipeline {
    device: Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl Pipeline {
    pub fn new(device: Device, image_format: vk::Format) -> Result<Self> {
        let descriptor_set_layout = make_descriptor_set_layout(&device)?;
        let layout = make_pipeline_layout(&device, descriptor_set_layout)?;
        // Se make_pipeline falhar, os dois layouts acima vazam: o processo encerra logo depois.
        let pipeline = make_pipeline(&device, layout, image_format)?;

        Ok(Self {
            device,
            descriptor_set_layout,
            layout,
            pipeline,
        })
    }

    /// Precisa ser bindado dentro de um renderpass.
    pub fn vk_pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }

    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        let raw = self.device.raw();
        unsafe {
            raw.destroy_pipeline(self.pipeline, None);
            raw.destroy_pipeline_layout(self.layout, None);
            raw.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

fn shader_code() -> Vec<u32> {
    // read_spv já valida o magic number e resolve o alinhamento de u32 — o que
    // no C++ era um reinterpret_cast<const u32*> torcendo para dar certo. Ele
    // continua necessário mesmo com os bytes embutidos: o `include_bytes!` dá um
    // `&[u8]` alinhado em 1, e o vkCreateShaderModule exige `[u32]`. A cópia
    // acontece uma vez, na criação da pipeline.
    //
    // `expect` e não `?`: estes bytes são constantes de compilação. Se não forem
    // SPIR-V válido, o build gerou um .spv quebrado — bug nosso, não condição de
    // ambiente que o chamador possa tratar.
    ash::util::read_spv(&mut Cursor::new(SHADER_SPV)).expect("o SPIR-V embutido é inválido")
}

fn make_descriptor_set_layout(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let bindings = [vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        // podemos ter um array de uniform buffers
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .stage_flags(vk::ShaderStageFlags::VERTEX)];

    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

    unsafe { device.raw().create_descriptor_set_layout(&info, None) }
        .context("criar descriptor set layout")
}

fn make_pipeline_layout(
    device: &Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<vk::PipelineLayout> {
    // Anexa o descriptor set layout do UBO para os shaders lerem `set = 0, binding = 0`.
    let set_layouts = [descriptor_set_layout];
    let info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);

    unsafe { device.raw().create_pipeline_layout(&info, None) }.context("criar pipeline layout")
}

fn make_pipeline(
    device: &Device,
    layout: vk::PipelineLayout,
    format: vk::Format,
) -> Result<vk::Pipeline> {
    let raw = device.raw();
    let shader_module = device.create_shader_module(&shader_code())?;

    let binding_descriptions = [Vertex::binding_description()];
    let attribute_descriptions = Vertex::attribute_descriptions();

    // Funções fixas
    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    // Viewport e scissor são dinâmicos (definidos no command buffer).
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
        // CCW: a projeção inverte Y (proj[1][1] *= -1) para casar com o clip
        // space do Vulkan, o que inverte o winding que o rasterizador enxerga.
        // As faces frontais viram CCW.
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

    // Necessário para dynamic rendering
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
        .layout(layout);

    // Sem cache por enquanto
    let created =
        unsafe { raw.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) };

    // O shader module só precisa viver até aqui, e some sozinho no fim da função
    // — inclusive no caminho de erro abaixo. O erro não é um VkResult puro
    // porque a criação pode falhar em parte das pipelines do lote.
    let pipelines = created
        .map_err(|(_, result)| result)
        .context("criar a graphics pipeline")?;

    Ok(pipelines[0])
}
