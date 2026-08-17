use glam::camera::rh::proj::vulkan::perspective as vulkan_perspective;
use glam::prelude::*;

pub struct Camera {
    projection: Mat4,
    fov: f32,
    aspect: f32,
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub(crate) fn new(fov: f32, position: Vec3, yaw: f32, pitch: f32, aspect: f32) -> Self {
        Self {
            projection: projection(fov, aspect),
            fov,
            aspect,
            position,
            yaw,
            pitch,
        }
    }

    pub(crate) fn matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(-self.position);
        let yaw = Mat4::from_rotation_y(self.yaw.to_radians());
        let pitch = Mat4::from_rotation_x(-self.pitch.to_radians());

        self.projection * pitch * yaw * translation
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn direction(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.to_radians().sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.to_radians().sin_cos();

        Vec3::new(cos_pitch * sin_yaw, sin_pitch, -cos_pitch * cos_yaw)
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn set_position(&mut self, value: Vec3) {
        self.position = value;
    }

    pub fn set_yaw(&mut self, value: f32) {
        self.yaw = value % 360.0;
    }

    pub fn set_pitch(&mut self, value: f32) {
        self.pitch = value.clamp(-90.0, 90.0);
    }

    pub(crate) fn set_aspect(&mut self, value: f32) {
        if value != self.aspect {
            self.aspect = value;
            self.projection = projection(self.fov, value);
        }
    }
}

fn projection(fov: f32, aspect: f32) -> Mat4 {
    let half_width = (fov.to_radians() / 2.0).tan();
    let vertical_fov = 2.0 * (half_width / aspect).atan();

    vulkan_perspective(vertical_fov, aspect, 0.1, 1000.0)
}
