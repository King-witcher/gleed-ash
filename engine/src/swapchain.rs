//! Equivalente a modules/engine/src/swapchain.{h,cc}.
//!
//! Diferença de modelagem: no C++ `AcquireNextImage` devolvia `SwapchainImage&`
//! e o `RenderPass` guardava essa referência. Em Rust isso brigaria com o
//! borrow checker (a mesma `Swapchain` precisa ser `&mut` depois, no present).
//! Como handles Vulkan são só inteiros opacos, `SwapchainImage` aqui é `Copy` e
//! sai por valor. Quem continua dono de verdade das ImageViews e dos semáforos
//! é a `Swapchain`.

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

/// O que a `Swapchain` de fato possui por imagem. A imagem em si é da swapchain
/// e some com ela; a view e o semáforo se destroem sozinhos.
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
    // A ORDEM DOS CAMPOS É A ORDEM DE DESTRUIÇÃO. As image views vêm das
    // imagens da swapchain, então morrem antes dela; e a swapchain antes da
    // surface. O `vk::raii` garante que nada disso passa do device, mas a ordem
    // entre irmãos continua sendo daqui.
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
        let present_queue = device.get_queue(device.present_index());
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
            // O semáforo é do frame in flight que o renderer acabou de esperar,
            // então não há nenhum acquire pendente nele.
            let result = unsafe {
                self.swapchain
                    .acquire_next_image(u64::MAX, image_available, vk::Fence::null())
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

        // A queue só é usada aqui e no submit do frame, ambos na mesma thread.
        let result = unsafe {
            self.swapchain
                .queue_present(&self.present_queue, &present_info)
        };

        match result {
            Ok(false) => {}
            // A imagem já foi consumida pela chamada de present; a recriação é
            // adiada para o próximo frame boundary, onde não existe nenhuma
            // referência por-frame para dentro da swapchain.
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.needs_recreate = true,
            Err(error) => return Err(error).context("present the swapchain image"),
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

    fn recreate(&mut self) -> Result<()> {
        self.device.wait_idle()?;

        let (swapchain, image_format, extent) =
            create_swapchain(&self.device, &self.surface, self.swapchain.handle())?;

        // Só depois de criar a nova é que as views da antiga podem morrer, e só
        // depois delas é que a antiga em si pode ser destruída — que é o que a
        // atribuição abaixo faz.
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
        // Não há como propagar de um Drop: se o wait falhou, destrói do mesmo jeito.
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

    // A extensão está em REQUIRED_EXTENSIONS e a surface veio da mesma instance
    // do device; `old` ou é nula ou é a swapchain que estamos substituindo.
    let swapchain =
        unsafe { device.vk().create_swapchain(&create_info) }.context("create swapchain")?;

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

            let image_view =
                unsafe { device.vk().create_image_view(&view_info) }.context("create image view")?;

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
    // Se width e height forem 0xFFFFFFFF, o tamanho da surface deveria ser
    // determinado pela extent da swapchain. Não suportamos surface dinâmica por
    // ora, então descartamos esse cenário.
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
