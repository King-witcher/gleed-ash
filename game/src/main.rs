mod geometry;
mod prelude;

use engine::error::Result;
use engine::{Engine, FrameContext, RunInfo};
use prelude::*;
use std::error::Error;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
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

    let axis = Vec3::new(0.0, 1.0, 1.0).normalize();

    let scene = vec![
        engine.upload_mesh(&geometry::cube(-0.45 * axis, 0.24))?,
        engine.upload_mesh(&geometry::truncated_icosahedron(0.45 * axis, 0.30))?,
    ];

    let run_info = RunInfo {
        update: Box::new(update),
        scene,
    };

    engine.run(run_info)
}

/// World units per second.
const MOVE_SPEED: f32 = 3.0;

fn update(frame: &mut FrameContext) {
    let mouse_delta = frame.input.mouse_delta();
    let camera = &mut frame.camera;

    camera.set_yaw(camera.yaw() + mouse_delta.x * 0.022 * 2.2);
    camera.set_pitch(camera.pitch() - mouse_delta.y * 0.022 * 2.2);

    let (sin_yaw, cos_yaw) = camera.yaw().to_radians().sin_cos();
    let forward = camera.direction();
    let right = Vec3::new(cos_yaw, 0.0, sin_yaw);

    let mut movement = Vec3::ZERO;

    if frame.input.is_key_down(Scancode::W) {
        movement += forward;
    }
    if frame.input.is_key_down(Scancode::S) {
        movement -= forward;
    }
    if frame.input.is_key_down(Scancode::D) {
        movement += right;
    }
    if frame.input.is_key_down(Scancode::A) {
        movement -= right;
    }

    camera.set_position(
        camera.position()
            + movement.normalize_or_zero() * MOVE_SPEED * frame.timestep.delta_time().as_secs_f32(),
    );
}
