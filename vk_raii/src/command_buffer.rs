use std::fmt;
use std::ops::Deref;

use ash::prelude::VkResult;
use ash::vk;

use crate::{CommandPool, Device};

/// Owns a `VkCommandBuffer` and frees it on drop.
///
/// Carries the pool it came from — `vkFreeCommandBuffers` needs it, and it is
/// also how the recording methods below reach the device dispatch table. None of
/// them check the command buffer state machine: `begin` before any `cmd_*`,
/// `end` before submitting, and no recording while a submit is pending are all
/// on the caller.
pub struct CommandBuffer {
    raw: vk::CommandBuffer,
    pool: CommandPool,
}

impl CommandBuffer {
    /// # Safety
    /// - `raw` must have been allocated from `pool`;
    /// - this takes ownership of `raw`: nothing else may free it.
    #[inline]
    pub unsafe fn from_raw(pool: CommandPool, raw: vk::CommandBuffer) -> Self {
        Self { raw, pool }
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandBuffer {
        self.raw
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
        unsafe { self.device().begin_command_buffer(self.raw, begin_info) }
    }

    /// # Safety
    /// The buffer must be in the recording state.
    #[inline]
    pub unsafe fn end(&mut self) -> VkResult<()> {
        unsafe { self.device().end_command_buffer(self.raw) }
    }

    /// # Safety
    /// The pool must have been created with `RESET_COMMAND_BUFFER`, and the
    /// buffer must not be pending execution.
    #[inline]
    pub unsafe fn reset(&mut self, flags: vk::CommandBufferResetFlags) -> VkResult<()> {
        unsafe { self.device().reset_command_buffer(self.raw, flags) }
    }
}

/// Every `cmd_*` below has the same contract: the buffer is recording, the pool
/// is externally synchronized, and the handles passed in are alive and valid for
/// the command.
impl CommandBuffer {
    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn begin_rendering(&mut self, rendering_info: &vk::RenderingInfo) {
        unsafe { self.device().cmd_begin_rendering(self.raw, rendering_info) };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn end_rendering(&mut self) {
        unsafe { self.device().cmd_end_rendering(self.raw) };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn pipeline_barrier2(&mut self, dependency_info: &vk::DependencyInfo) {
        unsafe {
            self.device()
                .cmd_pipeline_barrier2(self.raw, dependency_info)
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn set_viewport(&mut self, first_viewport: u32, viewports: &[vk::Viewport]) {
        unsafe {
            self.device()
                .cmd_set_viewport(self.raw, first_viewport, viewports)
        };
    }

    /// # Safety
    /// See the note on this `impl` block.
    #[inline]
    pub unsafe fn set_scissor(&mut self, first_scissor: u32, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device()
                .cmd_set_scissor(self.raw, first_scissor, scissors)
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
                .cmd_bind_pipeline(self.raw, bind_point, pipeline)
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
            self.device().cmd_bind_descriptor_sets(
                self.raw,
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
            self.device()
                .cmd_bind_vertex_buffers(self.raw, first_binding, buffers, offsets)
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
                .cmd_bind_index_buffer(self.raw, buffer, offset, index_type)
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
            self.device().cmd_draw(
                self.raw,
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
            self.device().cmd_draw_indexed(
                self.raw,
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
        unsafe { self.device().cmd_copy_buffer(self.raw, src, dst, regions) };
    }
}

impl Deref for CommandBuffer {
    type Target = vk::CommandBuffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl fmt::Debug for CommandBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandBuffer").field(&self.raw).finish()
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.pool
                .device()
                .free_command_buffers(self.pool.handle(), &[self.raw])
        };
    }
}
