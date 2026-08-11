//! Wrappers RAII finos sobre a `ash`, no espírito do `vulkan_raii.hpp`.
//!
//! Cada wrapper é dono do seu handle Vulkan, destrói ele no `Drop`, e faz
//! `Deref` para o objeto da `ash` por baixo — então a API inteira da `ash`
//! continua alcançável e nada precisa ser re-embrulhado método a método. O que
//! este módulo acrescenta são os construtores que devolvem um wrapper em vez de
//! um handle pelado, e os métodos que precisam de um pai que o tipo da `ash`
//! não carrega ([`PhysicalDevice`], [`CommandBuffer`]).
//!
//! Nada aqui valida uso do Vulkan. Os métodos são `unsafe` sempre que a chamada
//! que repassam é, e toda obrigação fica com quem chama: sincronização externa,
//! estado do objeto, pais coincidentes, parâmetros válidos.
//!
//! A única coisa que este módulo garante é **ordem de destruição**. Cada filho
//! guarda um handle refcontado do pai, então um `vkDestroy*` nunca roda depois
//! que o pai morreu — e nenhum lifetime vaza para a API, que é o que permite um
//! struct ser dono de um device e dos objetos criados a partir dele.
//!
//! Todo wrapper expõe `handle()`, devolvendo o handle da `ash` correspondente.

#[macro_use]
mod macros;

mod command_buffer;
mod command_pool;
mod device;
mod instance;
mod objects;
mod physical_device;
mod queue;
mod surface;
mod swapchain;

pub use command_buffer::CommandBuffer;
pub use command_pool::CommandPool;
pub use device::Device;
pub use instance::Instance;
pub use objects::{
    DescriptorPool, DescriptorSetLayout, Fence, ImageView, Pipeline, PipelineLayout, Semaphore,
    ShaderModule,
};
pub use physical_device::PhysicalDevice;
pub use queue::Queue;
pub use surface::Surface;
pub use swapchain::Swapchain;
