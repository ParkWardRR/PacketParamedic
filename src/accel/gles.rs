use anyhow::{Result, bail};

pub struct GlesBackend;

impl GlesBackend {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn execute_render_pass(
        &self,
        _fragment_shader_glsl: &str,
        _input_texture: &[u8],
    ) -> Result<Vec<u8>> {
        // TODO: Initialize EGL, create FBO, bind texture, render quad, read pixels
        bail!("GLES backend not implemented yet")
    }
}
