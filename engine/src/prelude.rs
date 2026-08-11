//! O que todo módulo do engine importa com `use crate::prelude::*`.
//!
//! Não reexporta o `vk::VkResult` de propósito: ele é o retorno **cru** de
//! cada chamada Vulkan e não deve escapar do ponto da chamada. O caminho dele
//! para o `Result` do engine é sempre o `.context(..)` do [`Context`], e nenhuma
//! função da crate devolve `VkResult` — se alguma voltasse a devolver, seria
//! sinal de que um erro está sendo repassado sem dizer qual chamada falhou.

pub use crate::error::{Context, Error, Result};
