//! Equivalente a modules/game/main.cc.
//!
//! O C++ separava em três alvos CMake (core / engine / game). Aqui é um crate
//! só: `core` (rust_types.h + panic.h) quase deixa de existir — u32/f32 e
//! `panic!` já são da linguagem, e o que sobrou de panic.h virou o
//! [`error`], que troca "abortar no lugar" por "propagar até aqui".

// Vários métodos existem só para espelhar a API pública das classes do C++
// (Window::GetSize, Input::WasMouseBtnPressed, RenderPass::Draw, ...) e ainda
// não têm chamador nesta demo — igual ao original.
#![allow(dead_code)]

mod allocator;
mod device;
mod engine;
mod error;
mod input;
mod mesh;
mod pipeline;
mod prelude;
mod renderer;
mod swapchain;
mod transfer;
mod vertex;
mod window;

// Só para trazer `source()` ao escopo; o nome `Error` continua sendo o nosso.
use std::error::Error as _;
use std::process::ExitCode;

use crate::prelude::*;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // `fn main() -> Result<..>` também funcionaria, mas imprimiria o
            // `Debug` do erro e nada da cadeia. Aqui o `Display` dá o contexto
            // ("falha ao criar swapchain") e o `source()` dá a causa real.
            eprintln!("erro fatal: {error}");

            let mut cause = error.source();
            while let Some(error) = cause {
                eprintln!("  causa: {error}");
                cause = error.source();
            }

            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut engine = engine::Engine::new()?;
    engine.run()
}
