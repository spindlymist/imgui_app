use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use image::{EncodableLayout, RgbaImage, imageops};

use crate::png_decoder::PngDecoder;

pub struct Textures<'a> {
    pub(crate) renderer: &'a mut imgui_wgpu::Renderer,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
}

pub struct Texture {
    pub texture_id: imgui::TextureId,
    pub width: f32,
    pub height: f32,
    pub intrinsic_width: f32,
    pub intrinsic_height: f32,
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

impl<'a> Textures<'a> {
    pub fn create_texture_from_file(&mut self, path: &Path) -> Result<Texture, CreateTextureError> {
        let max_size = self.device.limits().max_texture_dimension_2d;
        let (image, intrinsic_width, intrinsic_height) = load_and_scale_image(path, max_size)?;
        let width = image.width();
        let height = image.height();
        let raw_bytes = image.as_bytes();
        
        let texture_config = imgui_wgpu::TextureConfig {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            ..Default::default()
        };
        let texture = imgui_wgpu::Texture::new(self.device, self.renderer, texture_config);
        texture.write(self.queue, raw_bytes, width, height);
        let texture_id = self.renderer.textures.insert(texture);
        
        Ok(Texture {
            texture_id,
            width: width as f32,
            height: height as f32,
            intrinsic_width: intrinsic_width as f32,
            intrinsic_height: intrinsic_height as f32,
            is_downscaled: (width, height) != (intrinsic_width, intrinsic_height)
        })
    }
    
    pub fn delete_texture(&mut self, texture_id: imgui::TextureId) {
        // imgui_wgpu::Texture has a pointer to the underlying wgpu::Texture.
        // When wgpu::Texture is dropped, the texture is automatically destroyed.
        self.renderer.textures.remove(texture_id);
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
