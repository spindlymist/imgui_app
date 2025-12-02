#![allow(dead_code)]
// Adapted from image crate
// https://github.com/image-rs/image/blob/98b001da0ddcd91936a716696fba877df910b61d/src/codecs/png.rs

//! Decoding and Encoding of PNG Images
//!
//! PNG (Portable Network Graphics) is an image format that supports lossless compression.
//!
//! # Related Links
//! * <http://www.w3.org/TR/PNG/> - The PNG Specification

use std::io::{BufRead, Seek};

use image::{
    error::*,
    ColorType,
    ExtendedColorType,
    ImageDecoder,
    ImageError,
    ImageFormat,
    ImageResult,
    Limits,
    LimitSupport,
};

// http://www.w3.org/TR/PNG-Structure.html
// The first eight bytes of a PNG file always contain the following (decimal) values:
pub(crate) const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// PNG decoder
pub struct PngDecoder<R: BufRead + Seek> {
    color_type: ColorType,
    reader: png::Reader<R>,
    limits: Limits,
}

impl<R: BufRead + Seek> PngDecoder<R> {
    /// Creates a new decoder that decodes from the stream ```r```
    pub fn new(r: R) -> ImageResult<PngDecoder<R>> {
        Self::with_limits(r, Limits::no_limits())
    }

    /// Creates a new decoder that decodes from the stream ```r``` with the given limits.
    pub fn with_limits(r: R, limits: Limits) -> ImageResult<PngDecoder<R>> {
        limits.check_support(&LimitSupport::default())?;

        let max_bytes = usize::try_from(limits.max_alloc.unwrap_or(u64::MAX)).unwrap_or(usize::MAX);
        let mut decoder = png::Decoder::new_with_limits(r, png::Limits { bytes: max_bytes });
        
        decoder.set_ignore_text_chunk(true);
        decoder.set_ignore_iccp_chunk(true);
        decoder.ignore_checksums(true);

        let info = decoder.read_header_info().map_err(image_error_from_png)?;
        limits.check_dimensions(info.width, info.height)?;

        // By default the PNG decoder will scale 16 bpc to 8 bpc, so custom
        // transformations must be set. EXPAND preserves the default behavior
        // expanding bpc < 8 to 8 bpc.
        decoder.set_transformations(png::Transformations::EXPAND);
        let reader = decoder.read_info().map_err(image_error_from_png)?;
        let (color_type, bits) = reader.output_color_type();
        let color_type = match (color_type, bits) {
            (png::ColorType::Grayscale, png::BitDepth::Eight) => ColorType::L8,
            (png::ColorType::Grayscale, png::BitDepth::Sixteen) => ColorType::L16,
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => ColorType::La8,
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen) => ColorType::La16,
            (png::ColorType::Rgb, png::BitDepth::Eight) => ColorType::Rgb8,
            (png::ColorType::Rgb, png::BitDepth::Sixteen) => ColorType::Rgb16,
            (png::ColorType::Rgba, png::BitDepth::Eight) => ColorType::Rgba8,
            (png::ColorType::Rgba, png::BitDepth::Sixteen) => ColorType::Rgba16,

            (png::ColorType::Grayscale, png::BitDepth::One) => {
                return Err(unsupported_color(ExtendedColorType::L1))
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::One) => {
                return Err(unsupported_color(ExtendedColorType::La1))
            }
            (png::ColorType::Rgb, png::BitDepth::One) => {
                return Err(unsupported_color(ExtendedColorType::Rgb1))
            }
            (png::ColorType::Rgba, png::BitDepth::One) => {
                return Err(unsupported_color(ExtendedColorType::Rgba1))
            }

            (png::ColorType::Grayscale, png::BitDepth::Two) => {
                return Err(unsupported_color(ExtendedColorType::L2))
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Two) => {
                return Err(unsupported_color(ExtendedColorType::La2))
            }
            (png::ColorType::Rgb, png::BitDepth::Two) => {
                return Err(unsupported_color(ExtendedColorType::Rgb2))
            }
            (png::ColorType::Rgba, png::BitDepth::Two) => {
                return Err(unsupported_color(ExtendedColorType::Rgba2))
            }

            (png::ColorType::Grayscale, png::BitDepth::Four) => {
                return Err(unsupported_color(ExtendedColorType::L4))
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Four) => {
                return Err(unsupported_color(ExtendedColorType::La4))
            }
            (png::ColorType::Rgb, png::BitDepth::Four) => {
                return Err(unsupported_color(ExtendedColorType::Rgb4))
            }
            (png::ColorType::Rgba, png::BitDepth::Four) => {
                return Err(unsupported_color(ExtendedColorType::Rgba4))
            }

            (png::ColorType::Indexed, bits) => {
                return Err(unsupported_color(ExtendedColorType::Unknown(bits as u8)))
            }
        };

        Ok(PngDecoder {
            color_type,
            reader,
            limits,
        })
    }

    /// Returns the gamma value of the image or None if no gamma value is indicated.
    ///
    /// If an sRGB chunk is present this method returns a gamma value of 0.45455 and ignores the
    /// value in the gAMA chunk. This is the recommended behavior according to the PNG standard:
    ///
    /// > When the sRGB chunk is present, [...] decoders that recognize the sRGB chunk but are not
    /// > capable of colour management are recommended to ignore the gAMA and cHRM chunks, and use
    /// > the values given above as if they had appeared in gAMA and cHRM chunks.
    pub fn gamma_value(&self) -> ImageResult<Option<f64>> {
        Ok(self
            .reader
            .info()
            .source_gamma
            .map(|x| f64::from(x.into_scaled()) / 100_000.0))
    }

    // Turn this into an iterator over the animation frames.
    //
    // Reading the complete animation requires more memory than reading the data from the IDAT
    // frame–multiple frame buffers need to be reserved at the same time. We further do not
    // support compositing 16-bit colors. In any case this would be lossy as the interface of
    // animation decoders does not support 16-bit colors.
    //
    // If something is not supported or a limit is violated then the decoding step that requires
    // them will fail and an error will be returned instead of the frame. No further frames will
    // be returned.
    // pub fn apng(self) -> ImageResult<ApngDecoder<R>> {
    //     Ok(ApngDecoder::new(self))
    // }

    /// Returns if the image contains an animation.
    ///
    /// Note that the file itself decides if the default image is considered to be part of the
    /// animation. When it is not the common interpretation is to use it as a thumbnail.
    ///
    /// If a non-animated image is converted into an `ApngDecoder` then its iterator is empty.
    pub fn is_apng(&self) -> ImageResult<bool> {
        Ok(self.reader.info().animation_control.is_some())
    }
}

fn unsupported_color(ect: ExtendedColorType) -> ImageError {
    ImageError::Unsupported(UnsupportedError::from_format_and_kind(
        ImageFormat::Png.into(),
        UnsupportedErrorKind::Color(ect),
    ))
}

impl<R: BufRead + Seek> ImageDecoder for PngDecoder<R> {
    fn dimensions(&self) -> (u32, u32) {
        self.reader.info().size()
    }

    fn color_type(&self) -> ColorType {
        self.color_type
    }

    fn icc_profile(&mut self) -> ImageResult<Option<Vec<u8>>> {
        Ok(self.reader.info().icc_profile.as_ref().map(|x| x.to_vec()))
    }

    fn exif_metadata(&mut self) -> ImageResult<Option<Vec<u8>>> {
        Ok(self
            .reader
            .info()
            .exif_metadata
            .as_ref()
            .map(|x| x.to_vec()))
    }

    fn read_image(mut self, buf: &mut [u8]) -> ImageResult<()> {
        use byteorder_lite::{BigEndian, ByteOrder, NativeEndian};

        assert_eq!(u64::try_from(buf.len()), Ok(self.total_bytes()));
        self.reader.next_frame(buf).map_err(image_error_from_png)?;
        // PNG images are big endian. For 16 bit per channel and larger types,
        // the buffer may need to be reordered to native endianness per the
        // contract of `read_image`.
        // TODO: assumes equal channel bit depth.
        let bpc = self.color_type().bytes_per_pixel() / self.color_type().channel_count();

        match bpc {
            1 => (), // No reodering necessary for u8
            2 => buf.chunks_exact_mut(2).for_each(|c| {
                let v = BigEndian::read_u16(c);
                NativeEndian::write_u16(c, v);
            }),
            _ => unreachable!(),
        }
        Ok(())
    }

    fn read_image_boxed(self: Box<Self>, buf: &mut [u8]) -> ImageResult<()> {
        (*self).read_image(buf)
    }

    fn set_limits(&mut self, limits: Limits) -> ImageResult<()> {
        limits.check_support(&LimitSupport::default())?;
        let info = self.reader.info();
        limits.check_dimensions(info.width, info.height)?;
        self.limits = limits;
        // TODO: add `png::Reader::change_limits()` and call it here
        // to also constrain the internal buffer allocations in the PNG crate
        Ok(())
    }
}


fn image_error_from_png(err: png::DecodingError) -> ImageError {
    use png::DecodingError::*;
    match err {
        IoError(err) => ImageError::IoError(err),
        // The input image was not a valid PNG.
        err @ Format(_) => {
            ImageError::Decoding(DecodingError::new(ImageFormat::Png.into(), err))
        }
        // Other is used when:
        // - The decoder is polled for more animation frames despite being done (or not being animated
        //   in the first place).
        // - The output buffer does not have the required size.
        err @ Parameter(_) => ImageError::Parameter(ParameterError::from_kind(
            ParameterErrorKind::Generic(err.to_string()),
        )),
        LimitsExceeded => {
            ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory))
        }
    }
}
