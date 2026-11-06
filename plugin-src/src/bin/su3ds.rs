//! 変換単体の動作確認・ベンチマーク用 CLI。
//!
//! ```text
//! su3ds input.sog -o output.su3ds --max 30000 --no-dither
//! ```

use std::path::PathBuf;

use three_dgs_converter_su::convert::{ConvertOptions, convert_file};

fn main() {
    let mut args = std::env::args().skip(1);
    let input = match args.next() {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!(
                "usage: su3ds <input.sog|input.ply> [-o output.su3ds] [--max N] [--fit F] [--no-dither] [--preserve-quality] [--no-sh]"
            );
            std::process::exit(2);
        }
    };
    let mut output = input.with_extension("su3ds");
    let mut options = ConvertOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                output = PathBuf::from(args.next().expect("-o requires a path"));
            }
            "--max" => {
                options.max = args
                    .next()
                    .expect("--max requires a number")
                    .parse()
                    .expect("invalid --max");
            }
            "--fit" => {
                options.fit = args
                    .next()
                    .expect("--fit requires a number")
                    .parse()
                    .expect("invalid --fit");
            }
            "--scale" => {
                options.scale = args
                    .next()
                    .expect("--scale requires a number")
                    .parse()
                    .expect("invalid --scale");
            }
            "--flip-y" => options.flip_y = true,
            "--flip-z" => options.flip_z = true,
            "--no-center" => options.no_center = true,
            "--no-dither" => options.no_dither = true,
            "--preserve-quality" => options.preserve_quality = true,
            "--no-sh" => options.keep_sh = false,
            other => {
                eprintln!("unknown option: {other}");
                std::process::exit(2);
            }
        }
    }

    match convert_file(&input, &output, &options) {
        Ok(info) => {
            println!(
                "{} -> {} : {} -> {} points, {} ms, format={}",
                input.display(),
                output.display(),
                info.points_in_source,
                info.points_written,
                info.elapsed_ms,
                info.format
            );
            println!("load={} ms, process={} ms", info.load_ms, info.process_ms);
        }
        Err(error) => {
            eprintln!("convert failed: {error}");
            std::process::exit(1);
        }
    }
}
