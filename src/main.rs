#![feature(with_options)]

use std::io::{stdout, Write};

use clap::Arg;
use image::GenericImageView;

macro_rules! err_and_return {
    ($e: expr) => {{
        eprintln!("{}", $e);
        return;
    }}
}

static FILTERS: [&str; 5] = [
    "nearest",
    "triangle",
    "catmullrom",
    "gaussian",
    "lanczos3"
];

fn get_filter(filter: &str) -> Option<image::imageops::FilterType> {
    match filter {
        "nearest" => Some(image::imageops::Nearest),
        "triangle" => Some(image::imageops::Triangle),
        "catmullrom" => Some(image::imageops::CatmullRom),
        "gaussian" => Some(image::imageops::Gaussian),
        "lanczos3" => Some(image::imageops::Lanczos3),
        _ => None
    }
}

fn validate_size(size: String) -> Result<(), String> {
    if regex::Regex::new("\\d+[Xx]\\d+|term|original").unwrap().is_match(size.as_str()) {
        return Ok(());
    }
    Err("Size is not in a valid format".to_string())
}

fn get_size(size: &str) -> Option<(u32, u32)> {
    match size {
        "term" => {
            let sz = terminal_size::terminal_size()?;
            Some(((sz.0).0.into(), (sz.1).0.into()))
        }
        "original" => None,
        sz => {
            let re = regex::Regex::new("(\\d+)[Xx](\\d+)").unwrap();
            if re.is_match(sz) {
                let captures = re.captures(sz).unwrap();
                Some((captures.get(1).map_or(0, |m| m.as_str().parse().unwrap()),
                      captures.get(2).map_or(0, |m| m.as_str().parse().unwrap())))
            } else {
                let sz = terminal_size::terminal_size()?;
                Some(((sz.0).0.into(), (sz.1).0.into()))
            }
        }
    }
}

fn main() {
    let matches = clap::App::new("image_render")
        .version("1.0")
        .author("Riley Quinn")
        .about("A Rust thing that renders images.")
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .usage(format!("{} [--filters] -f filter [--size WxH|term|original] <input> [output]", std::env::args().next().unwrap()).as_str())
        .arg(Arg::with_name("filters")
            .long("filters")
            .help("List all resizing filters"))
        .arg(Arg::with_name("size")
            .short("s")
            .long("size")
            .default_value("term")
            .validator(validate_size)
            .help("Size of output image. Size must be WIDTHxHEIGHT, term, or original"))
        .arg(Arg::with_name("filter")
            .short("f")
            .long("filter")
            .possible_values(&FILTERS)
            .default_value("nearest")
            .help("Filter to use to resize image"))
        .arg(Arg::with_name("input")
            .index(1)
            .required_unless("filters")
            .help("Input file"))
        .arg(Arg::with_name("output")
            .index(2)
            .default_value("-")
            .help("Output file. Passing \"-\" will output to stdout"))
        .get_matches();

    if matches.is_present("filters") {
        for filter in FILTERS.iter() {
            println!("{}", filter);
        }
        return;
    }

    let mut img = match image::open(matches.value_of("input").unwrap()) {
        Ok(img) => img,
        Err(e) => err_and_return!(e)
    };

    let filter = get_filter(matches.value_of("filter").unwrap()).unwrap();

    // assume font ratio of 1:2.5
    // I may add something to deal with other ratios later
    img = img.resize_exact(img.width() * 3, img.height(), filter);

    if let Some(size) = get_size(matches.value_of("size").unwrap()) {
        img = img.resize(size.0, size.1, filter);
    }

    match matches.value_of("output") {
        Some("-") => write_image(img, &mut stdout()),
        Some(output) => write_image(img, &mut match std::fs::File::with_options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output) {
            Ok(f) => f,
            Err(e) => err_and_return!(e)
        }),
        _ => write_image(img, &mut stdout())
    };
}

fn write_image(img: image::DynamicImage, out: &mut impl Write) {
    // image is read left to right, top to bottom so storing y works. Find a better way?
    let mut last_y = 0;

    for pixel in img.pixels() {
        if pixel.1 != last_y {
            last_y = pixel.1;
            out.write_all(b"\n").unwrap();
        }

        let color = (pixel.2).0;

        // ANSI true color (8 bit RGB) for background: ESC[48;2;R;G;Bm
        out.write_all(format!("\x1b[48;2;{};{};{}m \x1b[0m", color[0], color[1], color[2]).as_bytes()).unwrap();
    }

    out.write_all(b"\n").unwrap();
}