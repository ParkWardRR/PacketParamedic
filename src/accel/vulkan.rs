use anyhow::{Result, Context, bail};
use ash::{Entry, Instance, Device};
use ash::vk;
use std::ffi::{CStr, CString};
use tracing::{info, warn, debug};

/// Safe-ish wrapper around Ash Vulkan handles.
/// Responsible for instance, device, and queue management.
pub struct VulkanBackend {
    _entry: Entry,
    instance: Instance,
    device: Device,
    #[allow(dead_code)]
    compute_queue: vk::Queue,
    #[allow(dead_code)]
    compute_queue_family_index: u32,
    #[allow(dead_code)]
    physical_device: vk::PhysicalDevice,
}

impl VulkanBackend {
    /// Initialize Vulkan backend (Headless Compute).
    /// Tries to load `libvulkan.so`, check for V3DV (or any device), and grab a compute queue.
    pub unsafe fn new() -> Result<Self> {
        // 1. Load Vulkan Entry
        let entry = match Entry::load() {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to load Vulkan library: {}", e);
                bail!("Vulkan library not found");
            }
        };

        // 2. Create Instance
        let app_name = CString::new("PacketParamedic").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(vk::API_VERSION_1_2); // Pi 5 V3DV supports 1.2

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info);
            // On macOS (MoltenVK) we might need portability enumeration, skipping for now
            // as target is strictly Linux/Pi 5.

        let instance = entry.create_instance(&create_info, None)
            .context("Failed to create Vulkan instance")?;

        // 3. Pick Physical Device
        let pdevices = instance.enumerate_physical_devices()
            .context("Failed to enumerate physical devices")?;
        
        let (pdevice, queue_family_index) = pdevices.iter().find_map(|&pdevice| {
            let props = instance.get_physical_device_properties(pdevice);
            let name = CStr::from_ptr(props.device_name.as_ptr());
            debug!("Found Vulkan device: {:?}", name);

            // Look for a queue family that supports COMPUTE
            let queue_families = instance.get_physical_device_queue_family_properties(pdevice);
            queue_families.iter().enumerate().find_map(|(index, info)| {
                if info.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                     Some((pdevice, index as u32))
                } else {
                    None
                }
            })
        }).ok_or_else(|| anyhow::anyhow!("No suitable Vulkan compute device found"))?;

         // 4. Create Logical Device
        let queue_priorities = [1.0];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info));
            // .enabled_features(...) // Add features like spirv_1_4 if needed

        let device = instance.create_device(pdevice, &device_create_info, None)
            .context("Failed to create logical device")?;

        // 5. Get Queue
        let compute_queue = device.get_device_queue(queue_family_index, 0);

        info!("Vulkan compute backend initialized on queue family {}", queue_family_index);

        Ok(Self {
            _entry: entry,
            instance,
            device,
            compute_queue,
            compute_queue_family_index: queue_family_index,
            physical_device: pdevice,
        })
    }

    /// Create a compute pipeline from SPIR-V bytecode.
    pub unsafe fn create_compute_pipeline(&self, spirv_code: &[u32]) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
        let shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(spirv_code);
        let shader_module = self.device.create_shader_module(&shader_module_create_info, None)?;
        
        let main_entry = CString::new("main").unwrap();
        
        let stage_create_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(&main_entry);

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default();
            // .set_layouts(&descriptor_set_layouts) ...
        let pipeline_layout = self.device.create_pipeline_layout(&pipeline_layout_create_info, None)?;

        let pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage_create_info)
            .layout(pipeline_layout);

        let pipelines = self.device.create_compute_pipelines(
            vk::PipelineCache::null(),
            std::slice::from_ref(&pipeline_create_info),
            None,
        ).map_err(|(_, e)| e)?;

        // Cleanup module (it's compiled into the pipeline now)
        self.device.destroy_shader_module(shader_module, None);
        
        Ok((pipelines[0], pipeline_layout))
    }
    /// Run a compute dispatch with a given pipeline and descriptor set.
    /// This is a simplified synchronous wrapper: Submit -> Wait.
    pub unsafe fn run_compute(
        &self,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
        group_count_x: u32,
    ) -> Result<()> {
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(self.compute_queue_family_index)
            .flags(vk::CommandPoolCreateFlags::TRANSIENT);
        
        let command_pool = self.device.create_command_pool(&command_pool_create_info, None)?;
        
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
            
        let command_buffers = self.device.allocate_command_buffers(&command_buffer_allocate_info)?;
        let command_buffer = command_buffers[0];
        
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
        self.device.begin_command_buffer(command_buffer, &begin_info)?;
        
        self.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );
        
        self.device.cmd_dispatch(command_buffer, group_count_x, 1, 1);
        
        self.device.end_command_buffer(command_buffer)?;
        
        let command_buffers_submit = [command_buffer];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&command_buffers_submit);
            
        let fence = self.device.create_fence(&vk::FenceCreateInfo::default(), None)?;
        
        self.device.queue_submit(self.compute_queue, &[submit_info], fence)?;
        
        // Wait for absolute completion (simple synchronous model for this appliance)
        self.device.wait_for_fences(&[fence], true, u64::MAX)?;
        
        self.device.destroy_fence(fence, None);
        self.device.destroy_command_pool(command_pool, None);
        
        Ok(())
    }
}

impl Drop for VulkanBackend {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
