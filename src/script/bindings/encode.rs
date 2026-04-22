//! `encode::*` static module — Rhai bindings for QR / DataMatrix / 1D
//! barcode generation. Wraps `src/encode.rs` primitives.

use crate::encode::{self, Format, OutputFormat};
use crate::script::convert::err;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult, Module};

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
        for name in ["qr", "datamatrix", "code128", "code39", "ean13", "upca"] {
            out.push(Dynamic::from(name.to_string()));
        }
        Ok(out)
    });

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
