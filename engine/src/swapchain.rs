//! Equivalent to modules/engine/src/swapchain.{h,cc}.
//!
//! Modeling difference: in C++ `AcquireNextImage` returned a `SwapchainImage&`
//! and the `RenderPass` held that reference. In Rust that would fight the
//! borrow checker (the same `Swapchain` must be `&mut` later, for present).
//! Since Vulkan handles are just opaque integers, `SwapchainImage` here is
//! `Copy` and goes out by value. The real owner of the ImageViews and the
//! semaphores is still the `Swapchain`.

use crate::device::Device;
use crate::internal_prelude::*;

/// Raw handles of one swapchain image, copyable because they are only opaque
/// integers. The real owner is the matching [`ImageResources`].
#[derive(Clone, Copy)]
pub struct SwapchainImage {
    pub index: u32,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    /// Signals that the renderer finished drawing and the image can be
    /// presented.
    pub render_finished: vk::Semaphore,
}

/// What the `Swapchain` actually owns per image. The image itself belongs to
/// the swapchain and goes away with it; the view and the semaphore destroy
/// themselves.
struct ImageResources {
    image: vk::Image,
    image_view: vk::raii::ImageView,
    render_finished: vk::raii::Semaphore,
}

impl ImageResources {
    fn handles(&self, index: u32) -> SwapchainImage {
        SwapchainImage {
            index,
            image: self.image,
            image_view: self.image_view.handle(),
            render_finished: self.render_finished.handle(),
        }
    }
}

pub struct Swapchain {
    // FIELD ORDER IS DESTRUCTION ORDER. The image views come from the
    // swapchain's images, so they die before it; and the swapchain before the
    // surface. `vk::raii` guarantees none of this outlives the device, but
    // sibling order is still ours.
    images: Vec<ImageResources>,
    swapchain: vk::raii::Swapchain,
    surface: vk::raii::Surface,
    present_queue: vk::raii::Queue,
    device: Device,
    image_format: vk::Format,
    extent: vk::Extent2D,
    needs_recreate: bool,
}

impl Swapchain {
    pub fn new(device: Device, surface: vk::raii::Surface) -> Result<Self> {
        let present_queue = device.queue(device.present_index());
        let (swapchain, image_format, extent) =
            create_swapchain(&device, &surface, vk::SwapchainKHR::null())?;
        let images = create_images(&device, &swapchain, image_format)?;

        Ok(Self {
            images,
            swapchain,
            surface,
            present_queue,
            device,
            image_format,
            extent,
            needs_recreate: false,
        })
    }

    pub fn acquire_next_image(&mut self, image_available: vk::Semaphore) -> Result<SwapchainImage> {
        loop {
            // The semaphore belongs to the frame in flight the renderer just
            // waited on, so no acquire is pending on it.
            let result = unsafe {
                self.swapchain
                    .acquire_next_image(u64::MAX, image_available, vk::Fence::null())
            };

            match result {
                Ok((index, suboptimal)) => {
                    // Suboptimal is a success code: the image WAS acquired and
                    // the semaphore WILL be signaled, so the frame must be
                    // rendered and presented normally. Recreation waits for the
                    // next frame boundary.
                    if suboptimal {
                        self.needs_recreate = true;
                    }
                    return Ok(self.images[index as usize].handles(index));
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    // No image was acquired and the semaphore was not signaled,
                    // so it is safe to retry with the same semaphore after
                    // recreating.
                    self.recreate()?;
                }
                Err(error) => return Err(error).context("acquire the next swapchain image"),
            }
        }
    }

    pub fn present(&mut self, image: SwapchainImage) -> Result<()> {
        let wait_semaphores = [image.render_finished];
        let swapchains = [self.swapchain.handle()];
        let image_indices = [image.index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        // The queue is only used here and in the frame's submit, both on the
        // same thread.
        let result = unsafe {
            self.swapchain
                .queue_present(&self.present_queue, &present_info)
        };

        match result {
            Ok(false) => {}
            // The image was already consumed by the present call; recreation is
            // deferred to the next frame boundary, where no per-frame reference
            // into the swapchain exists.
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.needs_recreate = true,
            Err(error) => return Err(error).context("present the swapchain image"),
        }

        Ok(())
    }

    /// Recreates the swapchain if a previous acquire/present reported it as
    /// out of date or suboptimal. Must be called at the frame boundary, before
    /// any reference into the swapchain images exists.
    pub fn recreate_if_needed(&mut self) -> Result<()> {
        if !self.needs_recreate {
            return Ok(());
        }
        self.needs_recreate = false;
        self.recreate()
    }

    pub fn image_format(&self) -> vk::Format {
        self.image_format
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    fn recreate(&mut self) -> Result<()> {
        self.device.wait_idle()?;

        let (swapchain, image_format, extent) =
            create_swapchain(&self.device, &self.surface, self.swapchain.handle())?;

        // Only after the new one exists may the old one's views die, and only
        // after them the old swapchain itself — which is what the assignment
        // below does.
        self.images.clear();
        self.swapchain = swapchain;
        self.image_format = image_format;
        self.extent = extent;
        self.images = create_images(&self.device, &self.swapchain, image_format)?;

        Ok(())
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        // There is no propagating from a Drop: if the wait failed, destroy
        // anyway.
        self.device.wait_idle().ok();
    }
}

fn create_swapchain(
    device: &Device,
    surface: &vk::raii::Surface,
    old: vk::SwapchainKHR,
) -> Result<(vk::raii::Swapchain, vk::Format, vk::Extent2D)> {
    let support = device.query_surface_support(surface)?;
    let format = choose_swap_surface_format(&support.formats);
    let present_mode = choose_swap_present_mode(&support.present_modes);
    let extent = choose_swap_extent(&support.capabilities)?;
    let min_image_count = choose_swap_image_count(&support.capabilities);

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .old_swapchain(old)
        .surface(surface.handle())
        .min_image_count(min_image_count)
        .image_format(format.format)
        .image_color_space(format.color_space)
        .image_extent(extent)
        // 1 because we are not doing stereoscopic 3D
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(support.capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        // Discards pixels obscured by other windows. May break blur effects,
        // though.
        .clipped(true);

    // The extension is in REQUIRED_EXTENSIONS and the surface came from the
    // same instance as the device; `old` is either null or the swapchain being
    // replaced.
    let swapchain = unsafe { device.vk_device().create_swapchain(&create_info) }
        .context("create the swapchain")?;

    Ok((swapchain, format.format, extent))
}

fn create_images(
    device: &Device,
    swapchain: &vk::raii::Swapchain,
    format: vk::Format,
) -> Result<Vec<ImageResources>> {
    let images = unsafe { swapchain.images() }.context("get the swapchain images")?;

    images
        .into_iter()
        .map(|image| {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let image_view = unsafe { device.vk_device().create_image_view(&view_info) }
                .context("create the image view")?;

            Ok(ImageResources {
                image,
                image_view,
                render_finished: device.create_semaphore()?,
            })
        })
        .collect()
}

fn choose_swap_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    for &format in formats {
        if format.format == vk::Format::B8G8R8A8_SRGB
            && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return format;
        }
    }
    formats[0]
}

fn choose_swap_present_mode(_present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    // for mode in present_modes {
    //     if *mode == vk::PresentModeKHR::MAILBOX {
    //         return *mode;
    //     }
    // }
    vk::PresentModeKHR::FIFO
}

fn choose_swap_extent(capabilities: &vk::SurfaceCapabilitiesKHR) -> Result<vk::Extent2D> {
    // If width and height are 0xFFFFFFFF, the surface size would be determined
    // by the swapchain extent. We do not support dynamic surfaces for now, so
    // this scenario is rejected.
    if (capabilities.current_extent.width | capabilities.current_extent.height) == u32::MAX {
        return Err(Error::unsupported("dynamic surface extent"));
    }
    Ok(capabilities.current_extent)
}

fn choose_swap_image_count(capabilities: &vk::SurfaceCapabilitiesKHR) -> u32 {
    if capabilities.max_image_count == 0 {
        return capabilities.min_image_count.max(3);
    }
    3u32.clamp(capabilities.min_image_count, capabilities.max_image_count)
}
