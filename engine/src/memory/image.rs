use crate::internal_prelude::*;
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

    pub fn create_image_view(&self, view_type: vk::ImageViewType) -> Result<vk::raii::ImageView> {
        unsafe {
            self.vk_image
                .create_image_view(
                    view_type,
                    self.format,
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                )
                .context("create image view")
        }
    }
}
