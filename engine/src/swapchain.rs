//! Equivalente a modules/engine/src/swapchain.{h,cc}.
//!
//! Diferença de modelagem: no C++ `AcquireNextImage` devolvia `SwapchainImage&`
//! e o `RenderPass` guardava essa referência. Em Rust isso brigaria com o
//! borrow checker (a mesma `Swapchain` precisa ser `&mut` depois, no present).
//! Como handles Vulkan são só inteiros opacos, `SwapchainImage` aqui é `Copy` e
//! sai por valor. Quem continua dono de verdade das ImageViews e dos semáforos
//! é a `Swapchain`, que os destrói no recreate e no `Drop`.

use ash::khr;
use ash::vk;

use crate::device::Device;
use crate::prelude::*;

/// Handles crus de uma imagem da swapchain, copiáveis por serem só inteiros
/// opacos. Quem é dono de verdade é o [`ImageResources`] correspondente.
#[derive(Clone, Copy)]
pub struct SwapchainImage {
    pub index: u32,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    /// Indica que o renderer terminou de desenhar e a imagem pode ser apresentada.
    pub render_finished: vk::Semaphore,
}

/// O que a `Swapchain` de fato possui por imagem. O semáforo é RAII (carrega um
/// clone do device); a image view ainda é destruída à mão.
struct ImageResources {
    image: vk::Image,
    image_view: vk::ImageView,
    render_finished: vk_raii::Semaphore,
}

impl ImageResources {
    fn handles(&self, index: u32) -> SwapchainImage {
        SwapchainImage {
            index,
            image: self.image,
            image_view: self.image_view,
            render_finished: self.render_finished.handle(),
        }
    }
}

pub struct Swapchain {
    device: Device,
    loader: khr::swapchain::Device,
    present_queue: vk_raii::Queue,
    /// Destruída depois da swapchain: o `Drop` abaixo roda antes dos campos.
    surface: vk_raii::Surface,
    image_format: vk::Format,
    extent: vk::Extent2D,
    handle: vk::SwapchainKHR,
    images: Vec<ImageResources>,
    needs_recreate: bool,
}

impl Swapchain {
    pub fn new(device: Device, surface: vk_raii::Surface) -> Result<Self> {
        let loader = khr::swapchain::Device::new(device.vulkan().raw(), device.raw());
        let present_queue = device.get_queue(device.present_index());

        let mut swapchain = Self {
            device,
            loader,
            present_queue,
            surface,
            image_format: vk::Format::UNDEFINED,
            extent: vk::Extent2D::default(),
            handle: vk::SwapchainKHR::null(),
            images: Vec::new(),
            needs_recreate: false,
        };
        swapchain.recreate()?;
        Ok(swapchain)
    }

    pub fn acquire_next_image(&mut self, image_available: vk::Semaphore) -> Result<SwapchainImage> {
        loop {
            let result = unsafe {
                self.loader.acquire_next_image(
                    self.handle,
                    u64::MAX,
                    image_available,
                    vk::Fence::null(),
                )
            };

            match result {
                Ok((index, suboptimal)) => {
                    // Suboptimal é código de sucesso: a imagem FOI adquirida e o
                    // semáforo SERÁ sinalizado, então o frame precisa ser
                    // renderizado e apresentado normalmente. A recriação espera
                    // o próximo frame boundary.
                    if suboptimal {
                        self.needs_recreate = true;
                    }
                    return Ok(self.images[index as usize].handles(index));
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    // Nenhuma imagem foi adquirida e o semáforo não foi
                    // sinalizado, então é seguro tentar de novo com o mesmo
                    // semáforo depois de recriar.
                    self.recreate()?;
                }
                Err(error) => return Err(error).context("adquirir a próxima imagem da swapchain"),
            }
        }
    }

    pub fn present(&mut self, image: SwapchainImage) -> Result<()> {
        let wait_semaphores = [image.render_finished];
        let swapchains = [self.handle];
        let image_indices = [image.index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let result = unsafe {
            self.loader
                .queue_present(self.present_queue.handle(), &present_info)
        };

        match result {
            Ok(false) => {}
            // A imagem já foi consumida pela chamada de present; a recriação é
            // adiada para o próximo frame boundary, onde não existe nenhuma
            // referência por-frame para dentro da swapchain.
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.needs_recreate = true,
            Err(error) => return Err(error).context("apresentar a imagem da swapchain"),
        }

        Ok(())
    }

    /// Recria a swapchain se um acquire/present anterior a reportou como out of
    /// date ou suboptimal. Precisa ser chamado no frame boundary, antes de
    /// existir qualquer referência para dentro das imagens da swapchain.
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

    /// Cria ou recria a swapchain.
    fn recreate(&mut self) -> Result<()> {
        self.device.wait_idle()?;

        let support = self.device.query_surface_support(&self.surface)?;
        let format = choose_swap_surface_format(&support.formats);
        let present_mode = choose_swap_present_mode(&support.present_modes);
        self.extent = choose_swap_extent(&support.capabilities)?;
        self.image_format = format.format;
        let min_image_count = choose_swap_image_count(&support.capabilities);

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .old_swapchain(self.handle)
            .surface(self.surface.handle())
            .min_image_count(min_image_count)
            .image_format(self.image_format)
            .image_color_space(format.color_space)
            .image_extent(self.extent)
            // 1 porque não estamos fazendo 3D estereoscópico
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            // Corta pixels obscurecidos por outras janelas. Pode bugar efeitos
            // de blur, porém.
            .clipped(true);

        let new_handle = unsafe { self.loader.create_swapchain(&create_info, None) }
            .context("criar swapchain")?;

        // Só depois de criar a nova é que a antiga (passada como old_swapchain)
        // pode ser destruída.
        self.destroy_images();
        if self.handle != vk::SwapchainKHR::null() {
            unsafe { self.loader.destroy_swapchain(self.handle, None) };
        }
        self.handle = new_handle;

        let images = unsafe { self.loader.get_swapchain_images(self.handle) }
            .context("obter as imagens da swapchain")?;
        self.create_images(&images)
    }

    fn create_images(&mut self, images: &[vk::Image]) -> Result<()> {
        self.images.clear();

        for &image in images {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(self.image_format)
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

            let image_view = unsafe { self.device.raw().create_image_view(&view_info, None) }
                .context("criar image view")?;

            self.images.push(ImageResources {
                image,
                image_view,
                render_finished: self.device.create_semaphore()?,
            });
        }

        Ok(())
    }

    fn destroy_images(&mut self) {
        let raw = self.device.raw();
        // O semáforo de cada imagem se destrói sozinho ao sair do vetor.
        for image in self.images.drain(..) {
            unsafe { raw.destroy_image_view(image.image_view, None) };
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        // Não há como propagar de um Drop: se o wait falhou, destrói do mesmo jeito.
        self.device.wait_idle().ok();
        self.destroy_images();
        if self.handle != vk::SwapchainKHR::null() {
            unsafe { self.loader.destroy_swapchain(self.handle, None) };
        }
    }
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
    // Se width e height forem 0xFFFFFFFF, o tamanho da surface deveria ser
    // determinado pela extent da swapchain. Não suportamos surface dinâmica por
    // ora, então descartamos esse cenário.
    if (capabilities.current_extent.width | capabilities.current_extent.height) == u32::MAX {
        return Err(Error::unsupported("extent de surface dinâmica"));
    }
    Ok(capabilities.current_extent)
}

fn choose_swap_image_count(capabilities: &vk::SurfaceCapabilitiesKHR) -> u32 {
    if capabilities.max_image_count == 0 {
        return capabilities.min_image_count.max(3);
    }
    3u32.clamp(capabilities.min_image_count, capabilities.max_image_count)
}
