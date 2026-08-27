use resource_manager::ImageResource;

use crate::{
    internal_prelude::*,
    memory::{AllocatedImage, TransferContext},
};

pub struct Texture {
    albedo: AllocatedImage,
    albedo_view: vk::raii::ImageView,
}

impl Texture {
    pub fn new(transfer: &mut TransferContext, albedo: ImageResource) -> Result<Self> {
        let albedo = transfer.upload_image(&albedo)?;
        let albedo_view = albedo.create_image_view(vk::ImageViewType::TYPE_2D)?;
        Ok(Self {
            albedo,
            albedo_view,
        })
    }
}
