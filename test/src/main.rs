use clap::Parser;
use clio::{InputPath, OutputPath};
use image::RgbaImage;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg()]
    input: InputPath,

    #[arg(short, long, default_value = "output.png")]
    output: OutputPath,
}

fn main() {
    let args = Args::parse();

    let in_image = match image::open(args.input.path().path()) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Image read error: {e}");
            return;
        }
    };

    let width = in_image.width();
    let height = in_image.height();
    let factor = 2;

    let rgba = RgbaImage::from(in_image);
    let out_rgba = xbrz::scale_rgba(&rgba, width as usize, height as usize, factor as usize);

    let out_width = width * factor;
    let out_height = height * factor;

    match image::save_buffer(
        args.output.path().path(),
        &out_rgba,
        out_width,
        out_height,
        image::ExtendedColorType::Rgba8,
    ) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error saving new image: {e}");
            return;
        }
    }

    println!(
        "Saved scaled image at {}",
        args.output.path().path().display()
    );
}
