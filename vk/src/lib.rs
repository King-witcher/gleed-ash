//! Vulkan para este workspace: a superfície da `ash` que de fato usamos, mais a
//! camada RAII em cima dela.
//!
//! Três coisas diferentes compartilham cada nome, e vale mantê-las separadas:
//!
//! - [`Device`] — o `VkDevice` cru, um inteiro opaco. Não sabe fazer nada
//!   sozinho.
//! - [`raw::Device`] — o device carregado da `ash`: esse handle mais a tabela
//!   de dispatch que o serve. É quem sabe chamar `vkCmdDraw`.
//! - [`raii::Device`] — o que este crate acrescenta: um `raw::Device` que é
//!   dono de si, mantém o pai vivo e se destrói sozinho.
//!
//! Nada fora daqui deve depender da `ash` diretamente — o que for preciso é
//! reexportado neste módulo.

pub mod raii;

/// Os objetos *carregados* da `ash`: um handle junto da tabela de dispatch que
/// o serve. Ficam num módulo à parte porque [`Device`] e [`Instance`], no raiz,
/// são os handles pelados.
pub mod raw {
    pub use ash::{Device, Instance};
}

pub use ash::prelude::VkResult;
pub use ash::vk::*;
pub use ash::{Entry, khr, util};
