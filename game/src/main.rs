mod geometry;
mod prelude;

use engine::Engine;
use engine::error::Result;
use prelude::*;
use std::error::Error;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // `fn main() -> Result<..>` would also work, but it would print
            // the error's `Debug` and nothing of the chain. Here `Display`
            // gives the context ("falha ao criar swapchain") and `source()`
            // the root cause.
            eprintln!("fatal error: {error}");

            let mut cause = error.source();
            while let Some(error) = cause {
                eprintln!("  cause: {error}");
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
