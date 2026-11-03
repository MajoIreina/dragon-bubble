use crate::{
    assets::{self, Skin},
    behavior::State,
};
use anyhow::{Context, Result, bail};
use image::{AnimationDecoder, ImageReader, RgbaImage};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io::BufReader, path::Path};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub frames: Vec<usize>,
    #[serde(default = "fps")]
    pub fps: f32,
    #[serde(default)]
    pub durations: Vec<f32>,
    #[serde(rename = "loop")]
    pub looping: Option<bool>,
}
fn fps() -> f32 {
    10.
}
pub struct SpritePack {
    pub frames: Vec<RgbaImage>,
    pub clips: HashMap<String, Clip>,
    pub mapping: HashMap<String, String>,
    pub size: [u32; 2],
    pub ground_offset: f32,
}
fn decode(path: &Path) -> Result<RgbaImage> {
    if assets::extension(path) == "svg" {
        let tree = resvg::usvg::Tree::from_data(
            &fs::read(path)?,
            &resvg::usvg::Options {
                resources_dir: path.parent().map(Path::to_path_buf),
                ..Default::default()
            },
        )?;
        let size = tree.size().to_int_size();
        if size.width() > 8192 || size.height() > 8192 {
            bail!("图片无法读取、内容为空或尺寸超过 8192 × 8192")
        }
        let mut pix = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
            .context("image allocation")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pix.as_mut(),
        );
        let mut pixels = pix.take();
        for p in pixels.chunks_exact_mut(4) {
            if p[3] > 0 {
                for i in 0..3 {
                    p[i] = ((p[i] as u32 * 255 / p[3] as u32).min(255)) as u8
                }
            }
        }
        Ok(RgbaImage::from_raw(size.width(), size.height(), pixels).context("SVG decode")?)
    } else {
        let mut reader = ImageReader::open(path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        Ok(reader.decode()?.to_rgba8())
    }
}
impl SpritePack {
    pub fn load(skin: &Skin) -> Result<Self> {
        assets::validate(skin)?;
        let path = assets::contained(&skin.root, skin.asset.as_deref().context("missing asset")?)?;
        let mut images = vec![];
        let mut clips = HashMap::new();
        if assets::extension(&path) == "gif" {
            let mut decoder =
                image::codecs::gif::GifDecoder::new(BufReader::new(fs::File::open(&path)?))?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(8192);
            limits.max_image_height = Some(8192);
            limits.max_alloc = Some(128 * 1024 * 1024);
            use image::ImageDecoder;
            decoder.set_limits(limits)?;
            let mut durations = vec![];
            let mut bytes = 0;
            for f in decoder.into_frames() {
                let f = f?;
                let (n, d) = f.delay().numer_denom_ms();
                durations.push((n as f32 / d as f32 / 1000.).max(0.01));
                bytes += f.buffer().len();
                if images.len() >= 256 || bytes > 128 * 1024 * 1024 {
                    bail!("序列帧为空或解码后超过 128 MB")
                }
                images.push(f.into_buffer())
            }
            clips.insert(
                "idle".into(),
                Clip {
                    frames: (0..images.len()).collect(),
                    fps: 10.,
                    durations,
                    looping: None,
                },
            );
        } else if let Some(frames) = skin.options.get("frames").and_then(|v| v.as_array()) {
            let mut bytes = 0;
            for f in frames {
                let img = decode(&assets::contained(
                    &skin.root,
                    f.as_str().context("invalid frame")?,
                )?)?;
                bytes += img.len();
                if bytes > 128 * 1024 * 1024 {
                    bail!("序列帧为空或解码后超过 128 MB")
                }
                images.push(img)
            }
        } else {
            let image = decode(&path)?;
            let grid = skin
                .options
                .get("grid")
                .and_then(|v| v.as_array())
                .map(|a| [a[0].as_u64().unwrap() as u32, a[1].as_u64().unwrap() as u32])
                .unwrap_or([1, 1]);
            if image.width() % grid[0] != 0 || image.height() % grid[1] != 0 {
                bail!("精灵图尺寸必须能被网格列数和行数整除")
            }
            let (w, h) = (image.width() / grid[0], image.height() / grid[1]);
            for y in 0..grid[1] {
                for x in 0..grid[0] {
                    images.push(image::imageops::crop_imm(&image, x * w, y * h, w, h).to_image())
                }
            }
        }
        if images.is_empty() {
            bail!("需要 1–256 张图片帧")
        }
        let mut left = u32::MAX;
        let mut top = u32::MAX;
        let mut right = 0;
        let mut bottom = 0;
        let mut dimensions = [0, 0];
        for img in &images {
            dimensions[0] = dimensions[0].max(img.width());
            dimensions[1] = dimensions[1].max(img.height());
            for (x, y, p) in img.enumerate_pixels() {
                if p.0[3] != 0 {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x + 1);
                    bottom = bottom.max(y + 1)
                }
            }
        }
        if right <= left || bottom <= top {
            bail!("序列帧为空或解码后超过 128 MB")
        }
        let size = [right - left, bottom - top];
        let mut frames = vec![];
        for img in images {
            let mut canvas = RgbaImage::new(dimensions[0], dimensions[1]);
            image::imageops::replace(&mut canvas, &img, 0, 0);
            frames.push(image::imageops::crop_imm(&canvas, left, top, size[0], size[1]).to_image())
        }
        if let Some(c) = skin.options.get("clips") {
            clips.extend(serde_json::from_value::<HashMap<String, Clip>>(c.clone())?)
        }
        if clips.is_empty() && frames.len() > 1 {
            clips.insert(
                "idle".into(),
                Clip {
                    frames: (0..frames.len()).collect(),
                    fps: 10.,
                    durations: vec![],
                    looping: None,
                },
            );
        }
        for (name, clip) in &clips {
            if clip.frames.iter().any(|&i| i >= frames.len()) {
                bail!("动作 {name} 的帧索引超出图片范围")
            }
        }
        let names = clips.keys().cloned().collect::<Vec<_>>();
        let mapping = crate::action::resolve(&names, &skin.animations);
        let ratio = 232. / size[0].max(size[1]) as f32;
        Ok(Self {
            frames,
            clips,
            mapping,
            size,
            ground_offset: 146. + size[1] as f32 * 0.5 * ratio,
        })
    }
    pub fn frame_at(&self, state: State, clock: f32, explicit: &HashMap<String, String>) -> usize {
        let key = self.mapping.get(state.key()).or_else(|| {
            if explicit.contains_key(state.key()) {
                None
            } else {
                self.mapping.get("idle")
            }
        });
        let Some(clip) = key.and_then(|k| self.clips.get(k)) else {
            return 0;
        };
        let duration = |i: usize| clip.durations.get(i).copied().unwrap_or(1. / clip.fps);
        let total = (0..clip.frames.len()).map(duration).sum::<f32>();
        if total <= 0. {
            return 0;
        }
        let looping = clip
            .looping
            .unwrap_or(state.looping() || !self.mapping.contains_key(state.key()));
        let mut at = if looping {
            clock.rem_euclid(total)
        } else {
            clock.min(total - 0.000001)
        };
        for (i, &frame) in clip.frames.iter().enumerate() {
            if at < duration(i) {
                return frame;
            }
            at -= duration(i)
        }
        *clip.frames.last().unwrap_or(&0)
    }
}
