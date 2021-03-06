// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[macro_use]
extern crate clap;
#[macro_use]
extern crate derive_error;
extern crate resvg;
extern crate fern;

use std::str::FromStr;
use std::fs;
use std::fmt;
use std::io::{
    self,
    Write,
};

use clap::{
    App,
    Arg,
    ArgMatches,
};

#[cfg(feature = "cairo-backend")]
use resvg::cairo;

use resvg::{
    log,
    svgdom,
    Document,
    FitTo,
    Options,
};

use svgdom::{
    ChainedErrorExt,
    WriteBuffer,
};

#[derive(Debug, Error)]
enum Error {
    Resvg(resvg::Error),
    Io(io::Error),

    #[cfg(feature = "cairo-backend")]
    Cairo(cairo::IoError),
}

fn main() {
    #[cfg(all(not(feature = "cairo-backend"), not(feature = "qt-backend")))]
    {
        eprintln!("Error: rendersvg has been build without any backend.");
        return;
    }

    if let Err(e) = process() {
        match e {
            Error::Resvg(ref e) => eprintln!("{}.", e.full_chain()),
            Error::Io(ref e) => eprintln!("Error: {}.", e),

            #[cfg(feature = "cairo-backend")]
            Error::Cairo(ref e) => eprintln!("Error: {}.", e),
        }

        std::process::exit(1);
    }
}

fn process() -> Result<(), Error> {
    fern::Dispatch::new()
        .format(log_format)
        .level(log::LogLevelFilter::Warn)
        .chain(std::io::stderr())
        .apply().unwrap();

    let app = prepare_app();
    let args = match app.get_matches_safe() {
        Ok(a) => a,
        Err(mut e) => {
            // change case before printing an error
            if e.message.starts_with("error:") {
                e.message = e.message.replace("error:", "Error:");
            }
            e.exit();
        }
    };

    let opt = fill_options(&args);

    let in_file  = args.value_of("in-svg").unwrap();
    let out_file = args.value_of("out-png").unwrap();

    // load file
    let doc = resvg::parse_doc_from_file(in_file, &opt)?;

    if args.is_present("dump-svg") {
        dump_svg(&doc, args.value_of("dump-svg").unwrap())?;
    }

    if args.is_present("pretend") {
        return Ok(());
    }

    match args.value_of("backend").unwrap() {
        "cairo" => {
            #[cfg(feature = "cairo-backend")]
            {
                let img = resvg::render_cairo::render_to_image(&doc, &opt)?;
                let mut buffer = fs::File::create(out_file)?;
                img.write_to_png(&mut buffer)?;
            }
        }
        "qt" => {
            #[cfg(feature = "qt-backend")]
            {
                let img = resvg::render_qt::render_to_image(&doc, &opt)?;
                img.save(out_file);
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn prepare_app<'a, 'b>() -> App<'a, 'b> {
    App::new("rendersvg")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("in-svg")
            .help("Input file")
            .required(true)
            .index(1)
            .validator(is_svg))
        .arg(Arg::with_name("out-png")
            .help("Output file")
            .required(true)
            .index(2)
            .validator(is_png))
        .arg(Arg::with_name("dpi")
            .long("dpi")
            .help("Sets the resolution [72..4000]")
            .value_name("DPI")
            .default_value("96")
            .validator(is_dpi))
        .arg(Arg::with_name("width")
            .short("w")
            .long("width")
            .value_name("LENGTH")
            .help("Sets the width in pixels")
            .conflicts_with_all(&["height", "zoom"])
            .validator(is_length))
        .arg(Arg::with_name("height")
            .short("h")
            .long("height")
            .value_name("LENGTH")
            .help("Sets the height in pixels")
            .conflicts_with_all(&["width", "zoom"])
            .validator(is_length))
        .arg(Arg::with_name("zoom")
            .short("z")
            .long("zoom")
            .value_name("ZOOM")
            .help("Zooms the image by a factor")
            .conflicts_with_all(&["width", "height"])
            .validator(is_zoom))
        .arg(Arg::with_name("background")
            .long("background")
            .value_name("COLOR")
            .help("Sets the background color")
            .validator(is_color))
        .arg(Arg::with_name("backend")
            .long("backend")
            .help("Sets the rendering backend")
            .takes_value(true)
            .default_value(default_backend())
            .possible_values(&backends()))
        .arg(Arg::with_name("dump-svg")
            .long("dump-svg")
            .help("Saves a preprocessed SVG to the selected file")
            .value_name("PATH"))
        .arg(Arg::with_name("pretend")
            .long("pretend")
            .help("Do all the steps except rendering"))
}

fn backends() -> Vec<&'static str> {
    let mut list = Vec::new();

    #[cfg(feature = "cairo-backend")]
    {
        list.push("cairo");
    }

    #[cfg(feature = "qt-backend")]
    {
        list.push("qt");
    }

    list
}

#[allow(unreachable_code)]
fn default_backend() -> &'static str {
    #[cfg(feature = "cairo-backend")]
    {
        return "cairo"
    }

    #[cfg(feature = "qt-backend")]
    {
        return "qt"
    }

    unreachable!();
}

fn is_svg(val: String) -> Result<(), String> {
    let val = val.to_lowercase();
    if val.ends_with(".svg") || val.ends_with(".svgz") {
        Ok(())
    } else {
        Err(String::from("The input file format must be SVG(Z)."))
    }
}

fn is_png(val: String) -> Result<(), String> {
    if val.ends_with(".png") || val.ends_with(".PNG") {
        Ok(())
    } else {
        Err(String::from("The output file format must be PNG."))
    }
}

fn is_dpi(val: String) -> Result<(), String> {
    let n = match val.parse::<u32>() {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    if n >= 72 && n <= 4000 {
        Ok(())
    } else {
        Err(String::from("Invalid DPI value."))
    }
}

fn is_length(val: String) -> Result<(), String> {
    let n = match val.parse::<u32>() {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    if n > 0 {
        Ok(())
    } else {
        Err(String::from("Invalid length value."))
    }
}

fn is_zoom(val: String) -> Result<(), String> {
    let n = match val.parse::<f32>() {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    if n > 0.0 {
        Ok(())
    } else {
        Err(String::from("Invalid zoom value."))
    }
}

fn is_color(val: String) -> Result<(), String> {
    match svgdom::types::Color::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => Err("Invalid color.".into()),
    }
}

fn fill_options(args: &ArgMatches) -> Options {
    let mut fit_to = FitTo::Original;
    if args.is_present("width") {
        fit_to = FitTo::Width(value_t!(args.value_of("width"), u32).unwrap());
    } else if args.is_present("height") {
        fit_to = FitTo::Height(value_t!(args.value_of("height"), u32).unwrap());
    } else if args.is_present("zoom") {
        fit_to = FitTo::Zoom(value_t!(args.value_of("zoom"), f32).unwrap());
    }

    let mut background = None;
    if args.is_present("background") {
        let s = args.value_of("background").unwrap();
        background = Some(svgdom::types::Color::from_str(s).unwrap());
    }

    Options {
        path: Some(args.value_of("in-svg").unwrap().into()),
        dpi: value_t!(args.value_of("dpi"), u16).unwrap() as f64,
        fit_to,
        background,
    }
}

fn dump_svg(doc: &Document, path: &str) -> Result<(), io::Error> {
    let mut f = fs::File::create(path)?;

    let mut opt = svgdom::WriteOptions::default();
    opt.indent = svgdom::Indent::Spaces(2);
    opt.attributes_indent = svgdom::Indent::Spaces(3);
    opt.write_hidden_attributes = true;

    let svgdoc = doc.to_svgdom();

    let mut out = Vec::new();
    svgdoc.write_buf_opt(&opt, &mut out);
    f.write_all(&out)?;

    Ok(())
}

fn log_format(out: fern::FormatCallback, message: &fmt::Arguments, record: &log::LogRecord) {
    use log::LogLevel;

    let lvl = match record.level() {
        LogLevel::Error => "Error",
        LogLevel::Warn => "Warning",
        LogLevel::Info => "Info",
        LogLevel::Debug => "Debug",
        LogLevel::Trace => "Trace",
    };

    out.finish(format_args!(
        "{} (in {}:{}): {}",
        lvl,
        record.target(),
        record.location().line(),
        message
    ))
}
