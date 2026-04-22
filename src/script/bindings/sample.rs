//! `sample::*` static module. Exposes the built-in sample-data registry
//! so scripts can discover what `--sample` names / formats / counts are
//! available. The CLI's generate path fetches from remote URLs; scripts
//! that actually need generated data should drive `http(...)` themselves
//! — this module surfaces the metadata + default URL so the wiring is
//! obvious.

use crate::sampledata::{self, SampleSpec};
use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Module};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    let _ = module.set_native_fn("list", || -> Result<Array, Box<EvalAltResult>> {
        let samples = sampledata::builtin_samples();
        let mut out = Array::new();
        let mut names: Vec<&String> = samples.keys().collect();
        names.sort();
        for name in names {
            let spec = &samples[name];
            out.push(Dynamic::from(spec_to_map(name, spec)));
        }
        Ok(out)
    });

    let _ = module.set_native_fn(
        "spec",
        |name: &str| -> Result<Map, Box<EvalAltResult>> {
            let samples = sampledata::builtin_samples();
            let spec = samples
                .get(name)
                .ok_or_else(|| err(format!("sample: unknown name '{name}'")))?;
            Ok(spec_to_map(name, spec))
        },
    );

    let _ = module.set_native_fn(
        "url",
        |name: &str, format: &str| -> Result<String, Box<EvalAltResult>> {
            let samples = sampledata::builtin_samples();
            let spec = samples
                .get(name)
                .ok_or_else(|| err(format!("sample: unknown name '{name}'")))?;
            spec.urls
                .get(format)
                .cloned()
                .ok_or_else(|| err(format!("sample: '{name}' has no '{format}' format")))
        },
    );

    engine.register_static_module("sample", module.into());
}

fn spec_to_map(name: &str, spec: &SampleSpec) -> Map {
    let mut m = Map::new();
    m.insert("name".into(), name.to_string().into());
    m.insert("description".into(), spec.description.clone().into());
    m.insert("default_format".into(), spec.default_format.clone().into());
    let formats: Array = spec
        .urls
        .keys()
        .cloned()
        .map(Dynamic::from)
        .collect();
    m.insert("formats".into(), formats.into());
    m.insert("count".into(), (spec.count as i64).into());
    m
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
    fn list_returns_nonempty() {
        let e = engine();
        let arr: Array = e.eval("sample::list()").expect("eval");
        assert!(!arr.is_empty(), "builtin samples should not be empty");
    }

    #[test]
    fn spec_has_expected_shape() {
        let e = engine();
        let arr: Array = e.eval("sample::list()").expect("eval");
        let first = arr[0].clone().try_cast::<Map>().unwrap();
        assert!(first.contains_key("name"));
        assert!(first.contains_key("description"));
        assert!(first.contains_key("default_format"));
        assert!(first.contains_key("formats"));
    }

    #[test]
    fn unknown_sample_throws() {
        let e = engine();
        let res: Result<Map, _> = e.eval(r#"sample::spec("definitely-not-a-sample")"#);
        assert!(res.is_err());
    }
}
