use resource_manager::ImageResource;

use crate::{
    device::Device,
    internal_prelude::*,
    memory::{AllocatedImage, TransferContext},
    Result,
};

pub struct Texture {
    image: AllocatedImage,
}

impl Texture {
    // pub fn new(
    //     transfer: &mut TransferContext,
    //     device: Device,
    //     image: ImageResource,
    // ) -> Result<Self> {
    //     let image_create_info = vk::ImageCreateInfo::default()
    //         .format(vk::Format::R8G8B8A8_SRGB)
    //         .extent(vk::Extent3D {
    //             width: image.width,
    //             height: image.height,
    //             depth: 1,
    //         })
    //         .mip_levels(1)
    //         .array_layers(1)
    //         .samples(vk::SampleCountFlags::TYPE_1)
    //         .tiling(vk::ImageTiling::OPTIMAL)
    //         .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
    //         .sharing_mode(vk::SharingMode::EXCLUSIVE);

    //     let image = unsafe { device.vk_device().create_image(&image_create_info) }
    //         .context("create texture image")?;

    //     let requirements = unsafe { image.memory_requirements() };
    // }
}
