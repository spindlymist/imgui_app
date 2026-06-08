use std::{
    collections::HashMap, fs, io, path::{Path, PathBuf}
};

use image::{EncodableLayout, RgbaImage, imageops};

use crate::png_decoder::PngDecoder;

pub struct Textures {
    pub(crate) textures: HashMap<u64, Texture>,
    pub(crate) unregistered_textures: Vec<u64>,
    pub(crate) next_id: u64,
    pub(crate) max_size: u32,
}

pub struct Texture {
    pub texture_data: dear_imgui_rs::OwnedTextureData,
    pub intrinsic_width: f32,
    pub intrinsic_height: f32,
    pub is_downscaled: bool,
}

impl Textures {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            textures: HashMap::new(),
            unregistered_textures: Vec::new(),
            next_id: 1,
            max_size: device.limits().max_texture_dimension_2d,
        }
    }
    
    pub fn register_textures(&mut self, imgui: &mut dear_imgui_rs::Context) {
        for id in self.unregistered_textures.drain(..) {
            let Some(texture) = self.textures.get_mut(&id) else { continue };
            imgui.register_user_texture(&mut texture.texture_data);
        }
    }
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

impl Textures {
    pub fn create_texture_from_file(&mut self, path: &Path) -> Result<u64, CreateTextureError> {
        let (image, intrinsic_width, intrinsic_height) = load_and_scale_image(path, self.max_size)?;
        let width = image.width();
        let height = image.height();
        let is_downscaled = width != intrinsic_width || height != intrinsic_height;
        
        let mut texture_data = dear_imgui_rs::OwnedTextureData::new();
        texture_data.create(dear_imgui_rs::TextureFormat::RGBA32, width, height);
        texture_data.set_data(image.as_bytes());
        
        let id = self.next_id;
        self.next_id += 1;
        self.textures.insert(id, Texture {
            texture_data,
            intrinsic_width: intrinsic_width as f32,
            intrinsic_height: intrinsic_height as f32,
            is_downscaled,
        });
        self.unregistered_textures.push(id);
        
        Ok(id)
    }
    
    pub fn get_texture(&self, id: u64) -> Option<&Texture> {
        self.textures.get(&id)
    }
    
    pub fn delete_texture(&mut self, id: u64) {
        self.textures.remove(&id);
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
