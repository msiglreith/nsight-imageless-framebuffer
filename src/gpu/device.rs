use crate::gpu;
use ash::{
    extensions::khr,
    version::{DeviceV1_0, DeviceV1_2, InstanceV1_0},
    vk,
};
use gpu_allocator::{VulkanAllocator, VulkanAllocatorCreateDesc};

pub struct Gpu {
    pub device: ash::Device,
    pub queue: vk::Queue,
    pub allocator: VulkanAllocator,
    pub cmd_pools: Vec<vk::CommandPool>,
    pub cmd_buffers: Vec<vk::CommandBuffer>,
    pub timeline: vk::Semaphore,
    frame_id: usize,
}

pub struct DescriptorsDesc {
    pub buffers: usize,
    pub images: usize,
    pub acceleration_structures: usize,
}

impl Gpu {
    pub unsafe fn new(
        instance: &gpu::Instance,
        frame_buffering: usize,
    ) -> anyhow::Result<Self> {
        let (device, queue) = {
            let device_extensions = vec![
                khr::Swapchain::name().as_ptr(),
            ];
            let features = vk::PhysicalDeviceFeatures::builder();
            let queue_priorities = [1.0];
            let queue_descs = [vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(instance.family_index)
                .queue_priorities(&queue_priorities)
                .build()];
            let device_desc = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_descs)
                .enabled_extension_names(&device_extensions)
                .enabled_features(&features);

            let device =
                instance
                    .instance
                    .create_device(instance.physical_device, &device_desc, None)?;
            let queue = device.get_device_queue(instance.family_index, 0);

            (device, queue)
        };

        let allocator = VulkanAllocator::new(&VulkanAllocatorCreateDesc {
            instance: instance.instance.clone(),
            device: device.clone(),
            physical_device: instance.physical_device,
            debug_settings: Default::default(),
        });

        let cmd_pools = (0..frame_buffering)
            .map(|_| {
                let desc =
                    vk::CommandPoolCreateInfo::builder().queue_family_index(instance.family_index);
                device.create_command_pool(&desc, None)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let cmd_buffers = {
            let cmd_buffers = cmd_pools
                .iter()
                .map(|pool| {
                    let desc = vk::CommandBufferAllocateInfo::builder()
                        .command_pool(*pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1);
                    device.allocate_command_buffers(&desc)
                })
                .collect::<Result<Vec<_>, _>>()?;
            cmd_buffers.into_iter().flatten().collect()
        };
        let timeline = {
            let mut timeline_desc = vk::SemaphoreTypeCreateInfo::builder()
                .semaphore_type(vk::SemaphoreType::TIMELINE)
                .initial_value(0);
            let desc = vk::SemaphoreCreateInfo::builder().push_next(&mut timeline_desc);
            device.create_semaphore(&desc, None)?
        };


        Ok(Self {
            device,
            queue,
            allocator,
            cmd_pools,
            cmd_buffers,
            timeline,
            frame_id: 0,
        })
    }

    pub unsafe fn create_layout(
        &mut self,
        num_constants: u32,
    ) -> anyhow::Result<gpu::Layout> {
        let push_constants = [vk::PushConstantRange::builder()
            .offset(0)
            .size(num_constants)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build()];

        let mut desc = vk::PipelineLayoutCreateInfo::builder();
        if num_constants > 0 {
            desc = desc.push_constant_ranges(&push_constants);
        }

        let pipeline_layout = self.create_pipeline_layout(&desc, None)?;

        Ok(gpu::Layout {
            pipeline_layout,
            samplers: gpu::GpuDescriptors {
                layout: vk::DescriptorSetLayout::null(),
                set: vk::DescriptorSet::null(),
            },
        })

    }

    pub unsafe fn acquire_cmd_buffer(&mut self) -> anyhow::Result<vk::CommandBuffer> {
        let frame_queue = self.cmd_pools.len();
        let frame_local = self.frame_id % frame_queue;
        if self.frame_id >= frame_queue {
            let semaphores = [self.timeline];
            let wait_values = [(self.frame_id - frame_queue + 1) as u64];
            let wait_info = vk::SemaphoreWaitInfo::builder()
                .semaphores(&semaphores)
                .values(&wait_values);
            self.device.wait_semaphores(&wait_info, !0)?;
            self.device.reset_command_pool(
                self.cmd_pools[frame_local],
                vk::CommandPoolResetFlags::empty(),
            )?;
        }

        let cmd_buffer = self.cmd_buffers[frame_local];
        let begin_desc = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        self.device.begin_command_buffer(cmd_buffer, &begin_desc)?;

        self.frame_id += 1;

        Ok(cmd_buffer)
    }

}

impl std::ops::Deref for Gpu {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
