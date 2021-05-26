mod device;
mod instance;
mod swapchain;

pub use self::device::{DescriptorsDesc, Gpu};
pub use self::instance::Instance;
pub use self::swapchain::Swapchain;

pub use ash::vk::{Buffer, BufferUsageFlags, ImageView, ImageLayout, AccelerationStructureKHR as AccelerationStructure};

use ash::vk;

#[derive(Debug, Copy, Clone)]
pub struct GpuDescriptors {
    pub layout: vk::DescriptorSetLayout,
    pub set: vk::DescriptorSet,
}

#[derive(Debug, Copy, Clone)]
pub struct Layout {
    pub pipeline_layout: vk::PipelineLayout,
    pub samplers: GpuDescriptors,
}
