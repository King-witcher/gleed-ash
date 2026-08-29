use std::fmt;

use ash::prelude::VkResult;
use ash::vk;

use super::{CommandPool, Device};

pub struct CommandBuffer {
    handle: vk::CommandBuffer,
    pool: CommandPool,
}

impl CommandBuffer {
    #[inline]
    pub unsafe fn from_handle(pool: CommandPool, handle: vk::CommandBuffer) -> Self {
        Self { handle, pool }
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandBuffer {
        self.handle
    }

    #[inline]
    pub fn pool(&self) -> &CommandPool {
        &self.pool
    }

    #[inline]
    pub fn device(&self) -> &Device {
        self.pool.device()
    }

    /// # Safety
    /// The buffer must be in the initial state and externally synchronized.
    #[inline]
    pub unsafe fn begin(&mut self, begin_info: &vk::CommandBufferBeginInfo) -> VkResult<()> {
        unsafe {
            self.device()
                .handle()
                .begin_command_buffer(self.handle, begin_info)
        }
    }

    /// # Safety
    /// The buffer must be in the recording state.
    #[inline]
    pub unsafe fn end(&mut self) -> VkResult<()> {
        unsafe { self.device().handle().end_command_buffer(self.handle) }
    }

    /// # Safety
    /// The pool must have been created with `RESET_COMMAND_BUFFER`, and the
    /// buffer must not be pending execution.
    #[inline]
    pub unsafe fn reset(&mut self, flags: vk::CommandBufferResetFlags) -> VkResult<()> {
        unsafe {
            self.device()
                .handle()
                .reset_command_buffer(self.handle, flags)
        }
    }
}

impl CommandBuffer {
    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn begin_rendering(&mut self, rendering_info: &vk::RenderingInfo) {
        unsafe {
            self.device()
                .handle()
                .cmd_begin_rendering(self.handle, rendering_info)
        };
    }

    #[inline]
    pub unsafe fn copy_buffer_to_image(
        &mut self,
        src_buffer: vk::Buffer,
        dst_image: vk::Image,
        dst_image_layout: vk::ImageLayout,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.device().handle().cmd_copy_buffer_to_image(
                self.handle,
                src_buffer,
                dst_image,
                dst_image_layout,
                regions,
            );
        }
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn end_rendering(&mut self) {
        unsafe { self.device().handle().cmd_end_rendering(self.handle) };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn pipeline_barrier2(&mut self, dependency_info: &vk::DependencyInfo) {
        unsafe {
            self.device()
                .handle()
                .cmd_pipeline_barrier2(self.handle, dependency_info)
        }
    }

    pub unsafe fn push_constants(
        &mut self,
        layout: vk::PipelineLayout,
        stage_flags: vk::ShaderStageFlags,
        offset: u32,
        constants: &[u8],
    ) {
        unsafe {
            self.device().handle().cmd_push_constants(
                self.handle,
                layout,
                stage_flags,
                offset,
                constants,
            )
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn set_viewport(&mut self, first_viewport: u32, viewports: &[vk::Viewport]) {
        unsafe {
            self.device()
                .handle()
                .cmd_set_viewport(self.handle, first_viewport, viewports)
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn set_scissor(&mut self, first_scissor: u32, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device()
                .handle()
                .cmd_set_scissor(self.handle, first_scissor, scissors)
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn bind_pipeline(
        &mut self,
        bind_point: vk::PipelineBindPoint,
        pipeline: vk::Pipeline,
    ) {
        unsafe {
            self.device()
                .handle()
                .cmd_bind_pipeline(self.handle, bind_point, pipeline)
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn bind_descriptor_sets(
        &mut self,
        bind_point: vk::PipelineBindPoint,
        layout: vk::PipelineLayout,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        unsafe {
            self.device().handle().cmd_bind_descriptor_sets(
                self.handle,
                bind_point,
                layout,
                first_set,
                descriptor_sets,
                dynamic_offsets,
            )
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn bind_vertex_buffers(
        &mut self,
        first_binding: u32,
        buffers: &[vk::Buffer],
        offsets: &[vk::DeviceSize],
    ) {
        unsafe {
            self.device().handle().cmd_bind_vertex_buffers(
                self.handle,
                first_binding,
                buffers,
                offsets,
            )
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn bind_index_buffer(
        &mut self,
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType,
    ) {
        unsafe {
            self.device()
                .handle()
                .cmd_bind_index_buffer(self.handle, buffer, offset, index_type)
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn draw(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device().handle().cmd_draw(
                self.handle,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            )
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device().handle().cmd_draw_indexed(
                self.handle,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            )
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn copy_buffer(
        &mut self,
        src: vk::Buffer,
        dst: vk::Buffer,
        regions: &[vk::BufferCopy],
    ) {
        unsafe {
            self.device()
                .handle()
                .cmd_copy_buffer(self.handle, src, dst, regions)
        };
    }
}

impl fmt::Debug for CommandBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandBuffer").field(&self.handle).finish()
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.pool
                .device()
                .handle()
                .free_command_buffers(self.pool.handle(), &[self.handle])
        };
    }
}
