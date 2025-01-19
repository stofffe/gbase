use clap::Parser;
use image::{GenericImage, GenericImageView, ImageEncoder, Rgba};
use std::{cmp, fs, io::Read};

#[derive(clap::Parser)]
struct Cli {
    #[clap(short, long)]
    destination: std::path::PathBuf,
    #[clap(short, long)]
    source: std::path::PathBuf,

    #[clap(long, default_value_t = 512)]
    width: u32,
    #[clap(long, default_value_t = 512)]
    height: u32,
}

#[derive(Debug)]
struct LoadFile {
    name: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    buffer: image::ImageBuffer<Rgba<u8>, Vec<u8>>,
}

fn main() {
    //
    // CLI
    //

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();
    let atlas_width = cli.width;
    let atlas_height = cli.height;
    let path = cli.source.clone();
    let mut atlas_path = cli.destination;

    //
    // load images and sort by height
    //

    let mut files = Vec::new();
    let mut stack = vec![path];
    while let Some(path) = stack.pop() {
        let metadata = fs::metadata(&path).expect("could not read metadata");
        if metadata.is_dir() {
            for entry in fs::read_dir(&path).expect("could not read dir") {
                let entry = entry.expect("could not open dir entry");
                stack.push(entry.path());
            }
        }

        if metadata.is_file() {
            let ext = path
                .extension()
                .expect("could not get file extensions")
                .to_str()
                .expect("could not convert from os string");
            if ext != "png" {
                log::warn!("ignoring non PNG file {:?}", path);
                return;
            }

            let img = image::ImageReader::open(&path)
                .expect("could not open image file")
                .decode()
                .expect("could not decode image file")
                .to_rgba8();
            let name = path
                .file_stem()
                .expect("could not get file name")
                .to_str()
                .expect("could not convert os string")
                .to_string();

            files.push(LoadFile {
                name,
                x: 0,
                y: 0,
                w: img.width(),
                h: img.height(),
                buffer: img,
            });

            log::info!("loaded file {:?}", path);
        }
    }

    //
    // packing
    //

    // sort by height -> scanline

    files.sort_by_key(|v| cmp::Reverse(v.buffer.height()));

    let mut shelf_height = 0;
    let mut x = atlas_width;
    let mut y = 0;
    for img in files.iter_mut() {
        let (width, height) = (img.buffer.width(), img.buffer.height());

        // bound check
        if x + width > atlas_width {
            x = 0;
            y += shelf_height;
            shelf_height = height;
        }
        if y + height > atlas_height {
            log::error!("ran out of space in texture atlas");
            return;
        }

        img.x = x;
        img.y = y;

        log::info!(
            "{}: origin {}, {} dim {}, {}",
            img.name,
            img.x,
            img.y,
            img.w,
            img.h
        );

        x += width;
    }

    //
    // create atlas
    //

    let mut atlas = image::RgbaImage::new(atlas_width, atlas_height);

    for img in files.iter() {
        atlas
            .copy_from(&img.buffer, img.x, img.y)
            .expect("could not write image to atlas");
    }

    atlas_path.set_extension("png");
    atlas.save(&atlas_path).expect("could not save file");

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            &atlas,
            atlas_width,
            atlas_height,
            image::ExtendedColorType::Rgba8,
        )
        .expect("could not write png to bytes");

    //
    // create rust file
    //

    let mut content = String::new();

    content.push_str(&format!("pub const ATLAS_WIDTH: u32 = {atlas_width};\n"));
    content.push_str(&format!("pub const ATLAS_HEIGHT: u32 = {atlas_height};\n"));

    content.push_str("pub struct AtlasSprite { pub x: u32, pub y: u32, pub w: u32, pub h: u32 }\n");

    for img in files.iter() {
        content.push_str(&format!(
            "pub const {}: AtlasSprite = AtlasSprite {{ x: {}, y: {}, w: {}, h: {} }};\n",
            img.name.to_uppercase(),
            img.x,
            img.y,
            img.w,
            img.h,
        ));
    }

    content.push_str("pub const ATLAS_BYTES: &[u8] = &[");

    for b in png_bytes {
        content.push_str(&format!("0x{b:X}, "));
    }

    content.push_str("];");

    atlas_path.set_extension("rs");
    fs::write(&atlas_path, content).expect("could not write rust file");
}
