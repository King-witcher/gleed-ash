use crate::memory::allocator::RaiiAllocation;

pub struct AllocatedImage {
    vk_image: vk::raii::Image,
    allocation: RaiiAllocation,
    format: vk::Format,
    extent: vk::Extent3D,
}

impl AllocatedImage {
    pub(super) fn from_parts(
        vk_image: vk::raii::Image,
        allocation: RaiiAllocation,
        format: vk::Format,
        extent: vk::Extent3D,
    ) -> Self {
        Self {
            vk_image,
            allocation,
            format,
            extent,
        }
    }

    pub(super) fn format(&self) -> vk::Format {
        self.format
    }

    pub(super) fn extent(&self) -> vk::Extent3D {
        self.extent
    }

    pub fn vk_image(&self) -> &vk::raii::Image {
        &self.vk_image
    }
}
