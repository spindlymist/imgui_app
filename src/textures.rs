use std::{
    collections::HashMap, fs, io, path::{Path, PathBuf}
};

use image::{EncodableLayout, RgbaImage, imageops};

use crate::png_decoder::PngDecoder;

pub struct TexturesPersistent {
    pub(crate) textures_by_id: HashMap<dear_imgui_rs::TextureId, TextureInfo>,
    pub(crate) max_size: u32,
}

pub struct Textures<'a> {
    pub(crate) persistent: &'a mut TexturesPersistent,
    pub(crate) renderer: &'a mut dear_imgui_wgpu::WgpuRenderer,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
}

pub struct TextureInfo {
    pub texture: wgpu::Texture,
    pub handle: dear_imgui_wgpu::ExternalTextureId,
    pub is_downscaled: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum CreateTextureError {
    #[error("File not found: `{}`.", .0.to_string_lossy())]
    NotFound(PathBuf),
    #[error("An IO error occurred while reading `{}`.", path.to_string_lossy())]
    Io {
        source: io::Error,
        path: PathBuf,
    },
    #[error("Couldn't decode the image at `{}`.", path.to_string_lossy())]
    Decode {
        source: image::ImageError,
        path: PathBuf,
    },
}

impl TexturesPersistent {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            textures_by_id: HashMap::new(),
            max_size: device.limits().max_texture_dimension_2d,
        }
    }
}

impl<'a> Textures<'a> {
    pub fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> dear_imgui_rs::TextureId {
        self.create_texture_from_bytes(width, height, data, false)
    }
    
    fn create_texture_from_bytes(&mut self, width: u32, height: u32, data: &[u8], is_downscaled: bool) -> dear_imgui_rs::TextureId {
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            texture_size,
        );
        
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let handle = self.renderer.register_external_texture(&texture_view).expect("Failed to register texture");
        let texture_id = handle.texture_id();
        let texture_info = TextureInfo {
            texture,
            handle,
            is_downscaled,
        };
        self.persistent.textures_by_id.insert(texture_id, texture_info);
        
        texture_id
    }
    
    pub fn create_texture_from_file(&mut self, path: &Path) -> Result<dear_imgui_rs::TextureId, CreateTextureError> {       
        let (image, intrinsic_width, intrinsic_height) = load_and_scale_image(path, self.persistent.max_size)?;
        let width = image.width();
        let height = image.height();
        let is_downscaled = width != intrinsic_width || height != intrinsic_height;
        
        let id = self.create_texture_from_bytes(width, height, image.as_bytes(), is_downscaled);
        Ok(id)
    }
    
    pub fn get_texture_info(&self, id: dear_imgui_rs::TextureId) -> Option<&TextureInfo> {
        self.persistent.textures_by_id.get(&id)
    }
    
    pub fn destroy_texture(&mut self, id: dear_imgui_rs::TextureId) {
        if let Some(texture_info) = self.persistent.textures_by_id.remove(&id) {
            let _ = self.renderer.unregister_external_texture(texture_info.handle);
            texture_info.texture.destroy();
        }
    }
    
    pub fn destroy_all_textures(&mut self) {
        for (_, texture_info) in self.persistent.textures_by_id.drain() {
            let _ = self.renderer.unregister_external_texture(texture_info.handle);
            texture_info.texture.destroy();
        }
    }
}

pub fn load_and_scale_image(path: &Path, max_size: u32) -> Result<(RgbaImage, u32, u32), CreateTextureError> {
    let decoder = {
        let file = fs::OpenOptions::new().read(true).open(path)
            .map_err(|source| {
                if source.kind() == io::ErrorKind::NotFound {
                    CreateTextureError::NotFound(path.to_owned())
                }
                else {
                    CreateTextureError::Io {
                        source,
                        path: path.to_owned(),
                    }
                }
            })?;
        PngDecoder::new(io::BufReader::new(file))
            .map_err(|source| CreateTextureError::Decode {
                source,
                path: path.to_owned()
            })?
    };

    let image_dynamic = image::DynamicImage::from_decoder(decoder)
        .map_err(|source| CreateTextureError::Decode {
            source,
            path: path.to_owned()
        })?;
    let image_rgba = match image_dynamic {
        image::DynamicImage::ImageRgba8(image_rgba8) => image_rgba8,
        _ => image_dynamic.to_rgba8(),
    };
    let width = image_rgba.width();
    let height = image_rgba.height();
    
    if width <= max_size && height <= max_size {
        Ok((image_rgba, width, height))
    }
    else {
        let (new_width, new_height) =
            if width >= height {
                let ratio = height as f64 / width as f64;
                let new_height = f64::round(ratio * max_size as f64) as u32;
                (max_size, new_height)
            }
            else {
                let ratio = width as f64 / height as f64;
                let new_width = f64::round(ratio * max_size as f64) as u32;
                (new_width, max_size)
            };
        let image_scaled = imageops::resize(&image_rgba, new_width, new_height, imageops::FilterType::CatmullRom);

        Ok((image_scaled, width, height))
    }
}

impl TextureInfo {
    pub fn width(&self) -> f32 {
        self.texture.width() as f32
    }
    
    pub fn height(&self) -> f32 {
        self.texture.height() as f32
    }
    
    pub fn size(&self) -> [f32; 2] {
        [self.width(), self.height()]
    }
}

impl<'tex> From<&TextureInfo> for dear_imgui_rs::TextureRef<'tex> {
    fn from(texture_info: &TextureInfo) -> Self {
        texture_info.handle.texture_id().into()
    }
}
