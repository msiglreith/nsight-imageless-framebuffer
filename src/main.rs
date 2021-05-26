use ash::{version::DeviceV1_0, vk};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use std::fs::File;

mod gpu;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(LogicalSize::new(1440.0, 900.0))
        .build(&event_loop)?;

    let size = window.inner_size();
    let frames_in_flight: usize = 2;

    unsafe {
        let instance = gpu::Instance::new(&window)?;
        let mut gpu = gpu::Gpu::new(
            &instance,
            frames_in_flight,
        )?;
        let mut wsi = gpu::Swapchain::new(&instance, &gpu, size.width, size.height)?;

        let mesh_pass = {
            let attachments = [
                vk::AttachmentDescription {
                    format: wsi.surface_format.format,
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                    ..Default::default()
                },
            ];
            let color_attachments = [vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            }];
            let subpasses = [vk::SubpassDescription::builder()
                .color_attachments(&color_attachments)
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .build()];
            let desc = vk::RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(&subpasses);
            gpu.create_render_pass(&desc, None)?
        };

        let mesh_fbo = {
            let image_formats0 = [wsi.surface_format.format];
            let images = [
                vk::FramebufferAttachmentImageInfo::builder()
                    .view_formats(&image_formats0)
                    .width(size.width as _)
                    .height(size.height as _)
                    .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .layer_count(1)
                    .build(),
            ];
            let mut attachments =
                vk::FramebufferAttachmentsCreateInfo::builder().attachment_image_infos(&images);
            let mut desc = vk::FramebufferCreateInfo::builder()
                .flags(vk::FramebufferCreateFlags::IMAGELESS)
                .render_pass(mesh_pass)
                .width(size.width as _)
                .height(size.height as _)
                .layers(1)
                .push_next(&mut attachments);
            desc.attachment_count = images.len() as _;
            gpu.create_framebuffer(&desc, None)?
        };


        let mesh_layout = gpu.create_layout(0)?;

        let mesh_vs = {
            let mut file = File::open("triangle.vert.spv")?;
            let code = ash::util::read_spv(&mut file)?;
            let desc = vk::ShaderModuleCreateInfo::builder().code(&code);
            gpu.create_shader_module(&desc, None)?
        };

        let mesh_fs = {
            let mut file = File::open("triangle.frag.spv")?;
            let code = ash::util::read_spv(&mut file)?;
            let desc = vk::ShaderModuleCreateInfo::builder().code(&code);
            gpu.create_shader_module(&desc, None)?
        };

        let mesh_pipeline = {
            let entry_vs = std::ffi::CStr::from_bytes_with_nul(b"main\0")?;
            let entry_fs = std::ffi::CStr::from_bytes_with_nul(b"main\0")?;
            let stages = [
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(mesh_vs)
                    .name(&entry_vs)
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(mesh_fs)
                    .name(&entry_fs)
                    .build(),
            ];

            let ia_desc = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let rasterizer_desc = vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false)
                .line_width(1.0);
            let viewport_desc = vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1);
            let multisample_desc = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let color_blend_attachment = [vk::PipelineColorBlendAttachmentState {
                blend_enable: vk::FALSE,
                src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
                dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ZERO,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
                color_write_mask: vk::ColorComponentFlags::all(),
            }];
            let color_blend_desc = vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&color_blend_attachment);
            let depth_stencil_desc = vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0);
            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_desc =
                vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);
            let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder();
            let desc = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&stages)
                .input_assembly_state(&ia_desc)
                .vertex_input_state(&vertex_input)
                .rasterization_state(&rasterizer_desc)
                .viewport_state(&viewport_desc)
                .multisample_state(&multisample_desc)
                .color_blend_state(&color_blend_desc)
                .depth_stencil_state(&depth_stencil_desc)
                .dynamic_state(&dynamic_desc)
                .render_pass(mesh_pass)
                .subpass(0)
                .layout(mesh_layout.pipeline_layout)
                .build(); // TODO

            gpu.create_graphics_pipelines(vk::PipelineCache::null(), &[desc], None)
                .unwrap()
        };

        let render_semaphores = (0..frames_in_flight)
            .map(|_| {
                let desc = vk::SemaphoreCreateInfo::builder();
                gpu.create_semaphore(&desc, None)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut frame_index = 0;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => *control_flow = ControlFlow::Exit,
                Event::LoopDestroyed => {
                    wsi.swapchain_fn.destroy_swapchain(wsi.swapchain, None);
                    instance.surface_fn.destroy_surface(instance.surface, None);
                }
                Event::MainEventsCleared => {
                    let frame_local = frame_index % frames_in_flight;

                    let image_index = wsi.acquire().unwrap();
                    let main_cmd_buffer = gpu.acquire_cmd_buffer().unwrap();

                    let clear_values = [
                        vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [0.0, 0.0, 0.0, 0.0],
                            },
                        },
                    ];

                    let attachments = [wsi.frame_rtvs[image_index]];
                    let mut render_pass_attachments =
                        vk::RenderPassAttachmentBeginInfo::builder().attachments(&attachments).build();
                    let mesh_pass_begin_desc = vk::RenderPassBeginInfo::builder()
                        .render_pass(mesh_pass)
                        .framebuffer(mesh_fbo)
                        .render_area(vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: vk::Extent2D {
                                width: size.width as _,
                                height: size.height as _,
                            },
                        })
                        .clear_values(&clear_values)
                        .push_next(&mut render_pass_attachments);
                    gpu.cmd_begin_render_pass(
                        main_cmd_buffer,
                        &mesh_pass_begin_desc,
                        vk::SubpassContents::INLINE,
                    );
                    gpu.cmd_bind_pipeline(
                        main_cmd_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        mesh_pipeline[0],
                    );
                    gpu.cmd_set_scissor(
                        main_cmd_buffer,
                        0,
                        &[vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: vk::Extent2D {
                                width: size.width as _,
                                height: size.height as _,
                            },
                        }],
                    );
                    gpu.cmd_set_viewport(
                        main_cmd_buffer,
                        0,
                        &[vk::Viewport {
                            x: 0.0,
                            y: size.height as _,
                            width: size.width as _,
                            height: -(size.height as f32),
                            min_depth: 0.0,
                            max_depth: 1.0,
                        }],
                    );
                    gpu.cmd_draw(main_cmd_buffer, 4, 1, 0, 0);

                    gpu.cmd_end_render_pass(main_cmd_buffer);

                    gpu.end_command_buffer(main_cmd_buffer).unwrap();

                    let main_waits = [wsi.frame_semaphores[image_index]];
                    let main_signals = [gpu.timeline, render_semaphores[frame_local]];
                    let main_stages = [vk::PipelineStageFlags::BOTTOM_OF_PIPE]; // TODO
                    let main_buffers = [main_cmd_buffer];

                    let main_waits_values = [0];
                    let main_signals_values = [frame_index as u64 + 1, 0];
                    let mut timeline_submit = vk::TimelineSemaphoreSubmitInfo::builder()
                        .wait_semaphore_values(&main_waits_values)
                        .signal_semaphore_values(&main_signals_values);
                    let main_submit = vk::SubmitInfo::builder()
                        .wait_semaphores(&main_waits)
                        .wait_dst_stage_mask(&main_stages)
                        .signal_semaphores(&main_signals)
                        .command_buffers(&main_buffers)
                        .push_next(&mut timeline_submit)
                        .build();
                    gpu.queue_submit(gpu.queue, &[main_submit], vk::Fence::null())
                        .unwrap();

                    let present_wait = [render_semaphores[frame_local]];
                    let present_swapchains = [wsi.swapchain];
                    let present_images = [image_index as u32];
                    let present_info = vk::PresentInfoKHR::builder()
                        .wait_semaphores(&present_wait)
                        .swapchains(&present_swapchains)
                        .image_indices(&present_images);
                    wsi.swapchain_fn
                        .queue_present(gpu.queue, &present_info)
                        .unwrap();

                    frame_index += 1;
                }
                _ => (),
            }
        })
    }
}
