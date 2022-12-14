use num::clamp;

use ash::{vk};
use ash::extensions::khr::{Surface, Swapchain};

use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, Window, WindowBuilder, WindowId},
};

use crate::renderer::core::Core;
use crate::renderer::logical_layer::LogicalLayer;
use crate::renderer::physical_layer::PhysicalLayer;

pub(crate) struct RenderTarget {
    pub(crate) swap_loader: Swapchain,
    pub(crate) swap_chain: vk::SwapchainKHR,
    pub(crate) surface_format: vk::Format,
    pub(crate) extent: vk::Extent2D,
    pub(crate) image_views: Vec<vk::ImageView>,
}

impl RenderTarget {
    pub(crate) fn new(core: &Core, physical_layer: &PhysicalLayer, logical_layer: &LogicalLayer) -> RenderTarget {
        fn choose_swap_extent(window: &Window, capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
            if capabilities.current_extent.width != u32::MAX {
                capabilities.current_extent
            }
            else {
                vk::Extent2D {
                    width: clamp(window.inner_size().width,
                                 capabilities.min_image_extent.width,
                                 capabilities.max_image_extent.width),
                    height: clamp(window.inner_size().height,
                                  capabilities.min_image_extent.height,
                                  capabilities.max_image_extent.height),
                }
            }
        }

        fn setup_image_views(logical_layer: &LogicalLayer, swap_loader: &Swapchain, swap_chain: vk::SwapchainKHR, surface_format: vk::Format) -> Vec<vk::ImageView> {
            let swap_chain_images: Vec<vk::Image>;
            unsafe {
                swap_chain_images = swap_loader
                    .get_swapchain_images(swap_chain).unwrap();
            }

            let mut image_views: Vec<vk::ImageView> = Vec::new();
            for i in swap_chain_images {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(i)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format)
                    .components(vk::ComponentMapping { // Allows remapping of color channels, I.E. turn all blues into shades of red
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY
                    })
                    .subresource_range(vk::ImageSubresourceRange { // Describes image purpose, I.E. a human
                        // viewable image for something like VR is composed of multiple images
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1
                    });

                unsafe {
                    image_views.push(  logical_layer.logical_device.create_image_view(&create_info, None).unwrap());
                }
            }

            return image_views;
        }

        let capabilities: vk::SurfaceCapabilitiesKHR;
        unsafe {
            capabilities = core.surface_loader
                .get_physical_device_surface_capabilities(physical_layer.physical_device,
                                                          core.surface).unwrap();
        }

        // Choose the first surface format with the specified conditions or choose the first option
        // otherwise
        let surface_format =
            match physical_layer
                .supported_surface_formats
                .iter()
                .find(|f|f.format == vk::Format::B8G8R8A8_SRGB &&
                    f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            {
                Some(x) => x,
                None => &physical_layer.supported_surface_formats[0]
            };

        let presentation_mode =
            match physical_layer
                .present_modes
                .iter()
                .find(|p|**p == vk::PresentModeKHR::MAILBOX)
            {
                Some(x) => *x,
                None => vk::PresentModeKHR::FIFO
            };

        let extent = choose_swap_extent(&core.window, &capabilities);

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count
        }

        let swap_create_info = vk::SwapchainCreateInfoKHR::default()
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1) // Always 1 except for stereoscopic 3D, I.E. VR
            .surface(core.surface)

            // TODO This assumes only one queue family. Consider adding support for separate queue
            // families later on
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)

            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT) // "It is also possible that you'll
            // render images to a separate image first to perform
            // operations like post-processing. In that case you may use a value like
            // VK_IMAGE_USAGE_TRANSFER_DST_BIT instead and use a memory operation to transfer the rendered
            // image to a swap chain image."
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(presentation_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        let swap_loader = Swapchain::new(&core.instance, &logical_layer.logical_device);
        let swap_chain: vk::SwapchainKHR;
        unsafe {
            swap_chain = swap_loader
                .create_swapchain(&swap_create_info, None).unwrap();
        }
        let image_views = setup_image_views(&logical_layer,
                                            &swap_loader,
                                            swap_chain,
                                            surface_format.format);

        return RenderTarget {
            swap_chain,
            swap_loader,
            surface_format: surface_format.format,
            extent,
            image_views
        }
    }

    pub(crate) fn destroy(&self, logical_layer: &LogicalLayer) {
        unsafe {
            for &v in self.image_views.iter() {
                logical_layer.logical_device.destroy_image_view(v, None);
            }

            self.swap_loader.destroy_swapchain(self.swap_chain, None);
        }
    }
}
