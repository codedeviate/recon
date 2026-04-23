//! `encode::*` static module — Rhai bindings for QR / DataMatrix / 1D
//! barcode generation. Wraps `src/encode.rs` primitives.

use crate::encode::{self, EncodeOptions, Format, OutputFormat, QrLevel};
use crate::script::convert::err;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult, Map, Module};

fn opts_from_map(m: &Map) -> Result<EncodeOptions, Box<EvalAltResult>> {
    let mut opts = EncodeOptions::default();
    if let Some(v) = m.get("qr_level") {
        let s = v.clone().into_string().map_err(|_| err("qr_level: string expected"))?;
        opts.qr_level = QrLevel::parse(&s).map_err(|e| err(e.to_string()))?;
    }
    Ok(opts)
}

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    let _ = module.set_native_fn(
        "encode",
        |format: &str, data: &str| -> Result<Blob, Box<EvalAltResult>> {
            let fmt = encode::parse_format(format).map_err(|e| err(e.to_string()))?;
            let matrix = encode::encode(fmt, data.as_bytes()).map_err(|e| err(e.to_string()))?;
            let png = encode::render_png(&matrix).map_err(|e| err(e.to_string()))?;
            Ok(png)
        },
    );

    let _ = module.set_native_fn(
        "encode",
        |format: &str, data: &str, out_format: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            let fmt = encode::parse_format(format).map_err(|e| err(e.to_string()))?;
            let out = encode::parse_output_format(out_format).map_err(|e| err(e.to_string()))?;
            let matrix = encode::encode(fmt, data.as_bytes()).map_err(|e| err(e.to_string()))?;
            Ok(match out {
                OutputFormat::Png => {
                    let bytes = encode::render_png(&matrix).map_err(|e| err(e.to_string()))?;
                    Dynamic::from(bytes)
                }
                OutputFormat::Svg => Dynamic::from(encode::render_svg(&matrix)),
                OutputFormat::Ascii => Dynamic::from(encode::render_ascii(&matrix)),
            })
        },
    );

    let _ = module.set_native_fn(
        "encode",
        |format: &str, data: &str, out_format: &str, opts: Map|
         -> Result<Dynamic, Box<EvalAltResult>> {
            let fmt = encode::parse_format(format).map_err(|e| err(e.to_string()))?;
            let out = encode::parse_output_format(out_format).map_err(|e| err(e.to_string()))?;
            let opts = opts_from_map(&opts)?;
            let matrix = encode::encode_with_opts(fmt, data.as_bytes(), &opts)
                .map_err(|e| err(e.to_string()))?;
            Ok(match out {
                OutputFormat::Png => {
                    let bytes = encode::render_png(&matrix).map_err(|e| err(e.to_string()))?;
                    Dynamic::from(bytes)
                }
                OutputFormat::Svg => Dynamic::from(encode::render_svg(&matrix)),
                OutputFormat::Ascii => Dynamic::from(encode::render_ascii(&matrix)),
            })
        },
    );

    // Convenience per-format helpers.
    let _ = module.set_native_fn(
        "qr",
        |data: &str| -> Result<Blob, Box<EvalAltResult>> {
            let m = encode::encode(Format::Qr, data.as_bytes()).map_err(|e| err(e.to_string()))?;
            encode::render_png(&m).map_err(|e| err(e.to_string()))
        },
    );
    let _ = module.set_native_fn(
        "datamatrix",
        |data: &str| -> Result<Blob, Box<EvalAltResult>> {
            let m = encode::encode(Format::DataMatrix, data.as_bytes())
                .map_err(|e| err(e.to_string()))?;
            encode::render_png(&m).map_err(|e| err(e.to_string()))
        },
    );
    let _ = module.set_native_fn(
        "barcode",
        |format: &str, data: &str| -> Result<Blob, Box<EvalAltResult>> {
            let fmt = encode::parse_format(format).map_err(|e| err(e.to_string()))?;
            // Accept any 1D format via parse_format; PNG output.
            let m = encode::encode(fmt, data.as_bytes()).map_err(|e| err(e.to_string()))?;
            encode::render_png(&m).map_err(|e| err(e.to_string()))
        },
    );

    let _ = module.set_native_fn("list", || -> Result<Array, Box<EvalAltResult>> {
        let mut out = Array::new();
        for name in [
            "qr", "datamatrix", "code128", "code39", "ean13", "upca",
            "aztec", "pdf417",
        ] {
            out.push(Dynamic::from(name.to_string()));
        }
        Ok(out)
    });

    // decode(blob) → #{ text, format } — scan a PNG/JPEG/WebP image
    // for a barcode, QR, DataMatrix, Aztec, PDF417, or MaxiCode.
    let _ = module.set_native_fn(
        "decode",
        |img: Blob| -> Result<rhai::Map, Box<EvalAltResult>> {
            let d = crate::decode::decode_bytes(&img, &[]).map_err(|e| err(e.to_string()))?;
            let mut m = rhai::Map::new();
            m.insert("text".into(), d.text.into());
            m.insert("format".into(), d.format.to_string().into());
            Ok(m)
        },
    );

    engine.register_static_module("encode", module.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn qr_returns_png_bytes() {
        let e = engine();
        let blob: Blob = e.eval(r#"encode::qr("hello")"#).expect("eval");
        // PNG header: 89 50 4E 47 0D 0A 1A 0A.
        assert!(blob.len() > 8);
        assert_eq!(&blob[..8], &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    }

    #[test]
    fn encode_ascii_returns_text() {
        let e = engine();
        let s: String = e
            .eval(r#"encode::encode("qr", "recon", "ascii")"#)
            .expect("eval");
        // ASCII QR renderers use block characters or '#'.
        assert!(!s.is_empty());
    }

    #[test]
    fn encode_svg_is_xml() {
        let e = engine();
        let s: String = e
            .eval(r#"encode::encode("qr", "x", "svg")"#)
            .expect("eval");
        assert!(s.starts_with("<?xml") || s.starts_with("<svg"));
    }

    #[test]
    fn unknown_format_throws() {
        let e = engine();
        let res: Result<Blob, _> = e.eval(r#"encode::barcode("bogus", "x")"#);
        assert!(res.is_err());
    }

    #[test]
    fn list_enumerates_formats() {
        let e = engine();
        let arr: Array = e.eval("encode::list()").expect("eval");
        assert!(arr.len() >= 6);
    }
}
