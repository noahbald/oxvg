use std::{io::Write as _, path::PathBuf};

use itertools::Itertools as _;
use skia_safe::{
    AlphaType, Color, ColorType, EncodedImageFormat, FontMgr, IPoint, ISize, ImageInfo, Surface,
    surfaces,
};

use crate::{args::RunCommand, config::Config, walk::Walk};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 512;
const MAX_DISTANCE: f64 = 510.0;

/// The result of visually regression testing input and output SVG
#[derive(Debug)]
pub enum Status {
    /// The output has visually regressed
    Broken(f64),
    /// The output is within threshold
    Ok,
}

#[derive(clap::Args, Debug)]
/// Visual regression testings.
///
/// Each SVG is rendered internally and visually compared to the original.
///
/// If the resulting SVG is visually regressed, it will an error will be emitted and screenshots
/// written to `./screenshots/`.
pub struct VisualRegression {
    #[clap(flatten)]
    /// Walk options
    pub walk: Walk,
}

impl RunCommand for VisualRegression {
    async fn run(self, _: Config) -> anyhow::Result<()> {
        self.walk()
    }
}

impl VisualRegression {
    /// Sets up directory walker and uses it to run visual regression on each file.
    ///
    /// This should be ran with the same walk options as provided to the original command.
    ///
    /// # Errors
    ///
    /// When invalid options are given
    pub fn walk(self) -> anyhow::Result<()> {
        self.walk.run(move || {
            Box::new(move |source, path, output| {
                let Some(output) = output else {
                    return;
                };
                let Ok(output) = std::fs::read_to_string(output) else {
                    eprintln!("`{}` missing for visual-regression test", output.display());
                    return;
                };
                if let Ok(Status::Broken(p)) = check(path, source, &output) {
                    eprintln!(
                        "\x1b[31mError: {}: visual regression detected by {p:.2}%.\x1b[0m",
                        if let Some(path) = path {
                            path.as_path().to_string_lossy()
                        } else {
                            std::borrow::Cow::Borrowed("document")
                        }
                    );
                }
            })
        })
    }
}

fn rasterise(
    mut dom: skia_safe::svg::Dom,
    resize: Option<ISize>,
) -> anyhow::Result<(ISize, ImageInfo, Surface)> {
    let scaled_size = resize.unwrap_or_else(|| ISize::new(WIDTH, HEIGHT));
    dom.set_container_size(scaled_size);

    let info = ImageInfo::new(scaled_size, ColorType::RGBA8888, AlphaType::Premul, None);
    let mut surface = surfaces::raster(&info, None, None)
        .ok_or_else(|| anyhow::Error::msg("Failed to create surface"))?;
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    dom.render(canvas);

    Ok((scaled_size, info, surface))
}

fn load_image(svg: &str, resize: Option<ISize>) -> anyhow::Result<(ISize, ImageInfo, Surface)> {
    rasterise(
        skia_safe::svg::Dom::from_str(svg, FontMgr::default())?,
        resize,
    )
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

fn difference(left: (u8, u8, u8, u8), right: (u8, u8, u8, u8)) -> f64 {
    if left == right {
        0.0
    } else {
        let dr = right.0 as f64 - left.0 as f64;
        let dg = right.1 as f64 - left.1 as f64;
        let db = right.2 as f64 - left.2 as f64;
        let da = right.3 as f64 - left.3 as f64;

        (dr * dr + dg * dg + db * db + da * da).sqrt() / MAX_DISTANCE
    }
}

#[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
fn compare(original: Vec<u8>, optimised: Vec<u8>) -> Status {
    assert_eq!(
        original.len(),
        optimised.len(),
        "The optimised render should have been resized to match the original"
    );
    assert_eq!(
        original.len() % 4,
        0,
        "image is not a quartet of RGBA values"
    );

    let len = original.len();
    let errors = original
        .into_iter()
        .tuple_windows()
        .zip(optimised.into_iter().tuple_windows())
        .map(|(left, right)| difference(left, right))
        .filter(|e| *e > 0.1)
        .count();
    let error_percentage = (errors as f64) / (len as f64);
    if error_percentage > 0.02 {
        Status::Broken(error_percentage * 100.0)
    } else {
        Status::Ok
    }
}

/// Compares the input SVG to the parsed/processed SVG and returns whether it has visually regressed.
///
/// Writes a screenshot to `./screenshots/<path>.png` if the input path is given and has visually regressed.
pub fn check(path: Option<&PathBuf>, original: &str, optimised: &str) -> anyhow::Result<Status> {
    let mut original_image = load_image(original, None)?;
    let mut optimised_image = load_image(optimised, Some(original_image.0))?;

    let original = draw_svg(&mut original_image)?;
    let optimised = draw_svg(&mut optimised_image)?;
    let result = compare(original, optimised);
    if matches!(result, Status::Broken(_))
        && let Some(path) = path
    {
        let path_string = path.to_string_lossy();
        let url_name = urlencoding::encode(&path_string);
        if let Some(data) =
            original_image
                .2
                .image_snapshot()
                .encode(None, EncodedImageFormat::PNG, None)
        {
            let mut output_path = PathBuf::new();
            output_path.push("screenshots");
            std::fs::create_dir_all(&output_path).ok();
            output_path.push(format!("{url_name}.input.png"));
            output_path.set_extension("png");
            let Ok(mut file) = std::fs::File::create(output_path) else {
                return Ok(result);
            };
            file.write_all(data.as_bytes()).ok();
        }
        if let Some(data) =
            optimised_image
                .2
                .image_snapshot()
                .encode(None, EncodedImageFormat::PNG, None)
        {
            let mut output_path = PathBuf::new();
            output_path.push("screenshots");
            std::fs::create_dir_all(&output_path).ok();
            output_path.push(format!("{url_name}.output.png"));
            let Ok(mut file) = std::fs::File::create(output_path) else {
                return Ok(result);
            };
            file.write_all(data.as_bytes()).ok();
        }
    }
    Ok(result)
}
