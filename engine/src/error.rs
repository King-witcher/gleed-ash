//! Não tem equivalente no C++: lá o `vulkan_raii.hpp` lançava `vk::SystemError`
//! sozinho a cada chamada que falhasse, ninguém capturava, e o erro subia até
//! terminar o processo. A `ash` é 1:1 com a API C e não lança nada — toda
//! chamada devolve `VkResult<T>`, então a escolha passa a ser explícita.
//!
//! A regra usada no engine inteiro é a mesma dos dois lados:
//!
//! - **Erro de ambiente** — não há GPU compatível, a SDL não abriu a janela, o
//!   driver ficou sem memória, o device foi perdido. Vira um [`Error`] e sobe
//!   com `?` até a `main`, que imprime a cadeia e sai com código de erro.
//! - **Bug nosso** — mapear um buffer alocado em VRAM pura, descartar um
//!   `RenderPass` sem submeter, embutir um SPIR-V inválido no binário.
//!   Continua sendo `panic!`/`expect`: não há nada que o chamador possa fazer,
//!   e a stack trace no ponto do erro vale mais do que um `Result`.
//!
//! O [`Context`] existe para não jogar fora a informação que os
//! `expect("Failed to create command pool")` originais carregavam:
//! `.context("criar command pool")?` propaga o erro E diz qual chamada falhou.

use std::fmt;

use ash::vk;
use gpu_allocator::AllocationError;

/// Mesmo padrão de `io::Result`: o alias esconde o tipo de erro, que é sempre
/// o mesmo em toda a crate.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// Uma chamada Vulkan devolveu um `VkResult` de erro.
    Vulkan {
        op: &'static str,
        result: vk::Result,
    },
    /// A `gpu-allocator` não conseguiu sub-alocar `VkDeviceMemory`.
    Allocation {
        op: &'static str,
        source: AllocationError,
    },
    /// A SDL falhou: init, subsistema de vídeo, janela, event pump ou surface.
    Sdl { op: &'static str, message: String },
    /// A máquina tem Vulkan, mas não atende a algum requisito deste engine.
    Unsupported(String),
}

impl Error {
    pub fn unsupported(what: impl Into<String>) -> Self {
        Error::Unsupported(what.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Vulkan { op, result } => write!(f, "falha ao {op}: {result}"),
            Error::Allocation { op, source } => write!(f, "falha ao {op}: {source}"),
            Error::Sdl { op, message } => write!(f, "falha ao {op}: {message}"),
            Error::Unsupported(what) => write!(f, "não suportado: {what}"),
        }
    }
}

impl std::error::Error for Error {
    /// Permite que a `main` caminhe pela cadeia (`source()`) e imprima a causa
    /// original junto do contexto de cada nível.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Vulkan { result, .. } => Some(result),
            Error::Allocation { source, .. } => Some(source),
            Error::Sdl { .. } | Error::Unsupported(_) => None,
        }
    }
}

/// Converte o erro de uma biblioteca em [`Error`], anexando o nome da operação.
///
/// É o que um `impl From<E> for Error` não conseguiria fazer: o `From` teria de
/// escolher um texto fixo por tipo de erro, e aqui o texto vem do ponto exato
/// da chamada. Em troca, `?` sozinho não basta — é sempre `.context(..)?`.
pub trait IntoError {
    fn into_error(self, op: &'static str) -> Error;
}

impl IntoError for vk::Result {
    fn into_error(self, op: &'static str) -> Error {
        Error::Vulkan { op, result: self }
    }
}

impl IntoError for AllocationError {
    fn into_error(self, op: &'static str) -> Error {
        Error::Allocation { op, source: self }
    }
}

// A SDL tem dois tipos de erro (o genérico e o do builder de janela) e ambos só
// carregam texto, então os dois desembocam na mesma variante.
impl IntoError for sdl3::Error {
    fn into_error(self, op: &'static str) -> Error {
        Error::Sdl {
            op,
            message: self.to_string(),
        }
    }
}

impl IntoError for sdl3::video::WindowBuildError {
    fn into_error(self, op: &'static str) -> Error {
        Error::Sdl {
            op,
            message: self.to_string(),
        }
    }
}

/// Substitui o `.expect("Failed to create command pool")` por
/// `.context("criar command pool")?`: mesma mensagem, só que o erro sobe em vez
/// de matar o processo no lugar.
pub trait Context<T> {
    fn context(self, op: &'static str) -> Result<T>;
}

impl<T, E: IntoError> Context<T> for std::result::Result<T, E> {
    fn context(self, op: &'static str) -> Result<T> {
        self.map_err(|error| error.into_error(op))
    }
}
