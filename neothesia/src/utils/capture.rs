//! Modo de captura para desenvolvimento: renderiza N frames e grava o ecrã num PNG.
//!
//! Variáveis de ambiente:
//! - `NEOTHESIA_CAPTURE=/caminho/img.png` ativa o modo
//! - `NEOTHESIA_CAPTURE_FRAME=120` frame a capturar (por omissão 120 ≈ 2 s)
//! - `NEOTHESIA_PAGE=library|settings|tracks` página inicial do menu
//! - `NEOTHESIA_MOUSE=x,y` posição lógica do rato (para ver estados de hover)

use std::path::PathBuf;

pub struct Capture {
    pub path: PathBuf,
    pub at_frame: u32,
    pub frame: u32,
}

impl Capture {
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("NEOTHESIA_CAPTURE")?;
        let at_frame = std::env::var("NEOTHESIA_CAPTURE_FRAME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120);
        Some(Self {
            path: PathBuf::from(path),
            at_frame,
            frame: 0,
        })
    }
}

pub fn env_page() -> Option<String> {
    std::env::var("NEOTHESIA_PAGE").ok()
}

pub fn env_mouse() -> Option<(f32, f32)> {
    let v = std::env::var("NEOTHESIA_MOUSE").ok()?;
    let (x, y) = v.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// Copia a textura para um buffer (deve ser chamada antes de `submit`).
pub fn encode_copy(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
) -> (wgpu::Buffer, u32) {
    let size = texture.size();
    let padded = (size.width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("capture"),
        size: (padded * size.height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(size.height),
            },
        },
        size,
    );
    (buffer, padded)
}

/// Lê o buffer (depois de `submit`) e grava o PNG.
pub fn save(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    padded: u32,
    texture: &wgpu::Texture,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    let size = texture.size();
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    let bgra = matches!(
        texture.format(),
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );

    let data = slice
        .get_mapped_range()
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let mut pixels = Vec::with_capacity((size.width * size.height * 4) as usize);
    for row in 0..size.height {
        let start = (row * padded) as usize;
        let line = &data[start..start + (size.width * 4) as usize];
        for px in line.chunks_exact(4) {
            if bgra {
                pixels.extend_from_slice(&[px[2], px[1], px[0], 255]);
            } else {
                pixels.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
    }
    drop(data);
    buffer.unmap();

    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size.width, size.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&pixels)?;
    log::info!("Captura gravada em {path:?}");
    Ok(())
}
