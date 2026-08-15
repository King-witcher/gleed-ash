//! Equivalent to modules/game/main.cc.
//!
//! C++ split this into three CMake targets (core / engine / game). Here it is
//! a single crate: `core` (rust_types.h + panic.h) all but disappears —
//! u32/f32 and `panic!` come with the language, and what was left of panic.h
//! became [`error`], which trades "abort on the spot" for "propagate up to
//! here". The game side is this file: it assembles the scene and hands it to
//! the engine.

// Several methods exist only to mirror the public API of the C++ classes
// (Window::GetSize, Input::WasMouseBtnPressed, RenderPass::Draw, ...) and have
// no caller in this demo yet — same as the original.
#![allow(dead_code)]

mod device;
mod engine;
mod error;
mod memory;
mod mesh;
mod platform;
mod prelude;
mod renderer;
mod swapchain;

// Only to bring `source()` into scope; `Error` still means our own.
use std::error::Error as _;
use std::process::ExitCode;

use glam::Vec3;

use crate::engine::Engine;
use crate::mesh::geometry;
use crate::prelude::*;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // `fn main() -> Result<..>` would also work, but it would print
            // the error's `Debug` and nothing of the chain. Here `Display`
            // gives the context ("falha ao criar swapchain") and `source()`
            // the root cause.
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
    let mut engine = Engine::new("Giuseppe")?;
    engine.window_mut().set_position(-1400, 200);

    // Both bodies sit ON the rotation axis, so they spin in place instead of
    // orbiting each other: the ball stays nearer the camera than the cube
    // forever. That is what keeps the scene correct while there is still no
    // depth buffer — each body is convex, so back-face culling resolves it
    // internally, and the farther one is simply drawn first.
    let axis = Vec3::new(0.0, 1.0, 1.0).normalize();

    let meshes = vec![
        engine.upload_mesh(&geometry::cube(-0.45 * axis, 0.24))?,
        engine.upload_mesh(&geometry::truncated_icosahedron(0.45 * axis, 0.30))?,
    ];

    engine.run(&meshes)
}
