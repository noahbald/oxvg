use std::{
    io::Write as _,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use skia_safe::{
    AlphaType, Color, ColorType, FontMgr, IPoint, ISize, ImageInfo, Surface, surfaces,
};

use crate::{args::RunCommand, config::Config, walk::Walk};

#[derive(clap::Args, Debug)]
/// Render SVG documents.
///
/// Each SVG is rendered as an SVG and written to the output as a PNG.
pub struct Render {
    #[clap(flatten)]
    /// Walk options
    pub walk: Walk,
}

impl RunCommand for Render {
    async fn run(self, _: Config) -> anyhow::Result<()> {
        self.walk()
    }
}

impl Render {
    /// Sets up directory walker and uses it to run visual regression on each file.
    ///
    /// This should be ran with the same walk options as provided to the original command.
    ///
    /// # Errors
    ///
    /// When invalid options are given
    pub fn walk(self) -> anyhow::Result<()> {
        let error = Arc::new(AtomicBool::new(false));
        self.walk.run(|| {
            let error = Arc::clone(&error);
            Box::new(move |source, path, output| {
                let Some(output) = output.or(path) else {
                    eprintln!("Rendering to stdout not supported");
                    error.store(true, Ordering::Relaxed);
                    return;
                };
                let mut output = output.clone();
                output.set_extension("png");
                let image = load_image(source).map_err(anyhow::Error::msg);
                let data = image.and_then(|mut i| draw_svg(&mut i).map_err(anyhow::Error::msg));
                let result = data.and_then(|d| {
                    if let Some(parent) = output.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let mut file = std::fs::File::create(output)?;
                    file.write_all(&d)?;
                    Ok(())
                });
                if let Err(err) = result {
                    eprintln!("Error: {err}");
                    error.store(true, Ordering::Relaxed);
                }
            })
        })?;
        if error.load(Ordering::Relaxed) {
            Err(anyhow::anyhow!("Failed to format all documents!"))
        } else {
            Ok(())
        }
    }
}

fn rasterise(dom: &skia_safe::svg::Dom) -> anyhow::Result<(ISize, ImageInfo, Surface)> {
    let size = dom.root().intrinsic_size().to_round();
    let size = if size.width == 0 || size.height == 0 {
        ISize::new(512, 512)
    } else {
        size
    };
    let info = ImageInfo::new(size, ColorType::RGBA8888, AlphaType::Premul, None);
    let mut surface = surfaces::raster(&info, None, None)
        .ok_or_else(|| anyhow::Error::msg("Failed to create surface"))?;
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    dom.render(canvas);

    Ok((size, info, surface))
}

fn load_image(svg: &str) -> anyhow::Result<(ISize, ImageInfo, Surface)> {
    rasterise(&skia_safe::svg::Dom::from_str(svg, FontMgr::default())?)
}

#[allow(clippy::cast_sign_loss)]
fn draw_svg((size, info, surface): &mut (ISize, ImageInfo, Surface)) -> anyhow::Result<Vec<u8>> {
    let mut pixels = vec![0u8; (size.area() * 4) as usize];
    if surface.read_pixels(
        info,
        &mut pixels,
        (size.width * 4) as usize,
        IPoint::new(0, 0),
    ) {
        Ok(pixels)
    } else {
        Err(anyhow::Error::msg("Failed to read pixels from surface"))
    }
}
