//! Guard rail: bit-identical results across platforms need every
//! platform-dependent float function to go through `src/fmath.rs`.

use std::fs;
use std::path::Path;

/// (`log` is left out: `SaveFile::log()` is a plain accessor.)
/// `std` float methods whose result may differ in the last bit between
/// platforms (libm) or optimisation levels (`powi`, `mul_add`).
const FORBIDDEN: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "exp", "exp2", "exp_m1", "ln",
    "ln_1p", "log2", "log10", "powf", "powi", "cbrt", "hypot", "sin_cos", "mul_add",
];

fn scan(dir: &Path, hits: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            scan(&path, hits);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") || path.file_name().and_then(|n| n.to_str()) == Some("fmath.rs") {
            continue;
        }
        for (i, line) in fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for name in FORBIDDEN {
                let method = format!(".{name}(");
                let qualified = [format!("f32::{name}("), format!("f64::{name}(")];
                if code.contains(&method) || qualified.iter().any(|q| code.contains(q.as_str())) {
                    hits.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
                }
            }
        }
    }
}

#[test]
fn no_platform_dependent_float_functions_outside_fmath() {
    let mut hits = Vec::new();
    scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut hits);
    assert!(
        hits.is_empty(),
        "raw libm-dependent float calls in dendro-core/src (use crate::fmath::DMath: dsin, dcos, dexp, dpowf, dpowi, ...):\n{}",
        hits.join("\n")
    );
}
