//! Compiles all slang shaders in src/shaders

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;

const SHADERS_DIR: &str = "src/shaders";
const ENTRY_POINTS: &'static [&'static str] = &["vertMain", "fragMain"];

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shader_dir = root.join(SHADERS_DIR);

    println!("cargo::rerun-if-changed={}", shader_dir.display());

    let paths = list_shaders(&shader_dir).expect("failed to collect shaders");

    for shader in &paths {
        println!("cargo::rerun-if-changed={}", shader.display());
    }

    let jobs: Vec<_> = paths
        .into_iter()
        .map(|path| thread::spawn(move || (compile(&path), path)))
        .collect();

    let total = jobs.len();
    let mut failures = 0;

    for job in jobs {
        let (result, path) = job.join().expect("compilation thread panicked");
        let shader = path.strip_prefix(&root).unwrap().display();

        match result {
            Ok(output) if output.status.success() => {
                warn(format!("[SUCCESS] {shader} -> {shader}.spv"));
                report_diagnostics(&output);
            }
            Ok(output) => {
                failures += 1;
                warn(format!("[FAILED] {shader} ({})", output.status));
                report_diagnostics(&output);
            }
            Err(e) => {
                failures += 1;
                warn(format!("[FAILED] {shader}: unable to run slangc: {e}"));
            }
        }
    }

    if failures > 0 {
        panic!("slangc: {failures} out of {total} shader(s) failed");
    }

    warn(format!("slangc: {total} shader(s) compiled"));
}

fn compile(shader: &Path) -> io::Result<Output> {
    let spv = shader.with_extension("spv");

    let mut cmd = Command::new("slangc");
    cmd.arg(shader).args([
        "-target",
        "spirv",
        "-profile",
        "spirv_1_4",
        "-emit-spirv-directly",
        "-fvk-use-entrypoint-name",
    ]);

    for entry in ENTRY_POINTS {
        cmd.args(["-entry", entry]);
    }

    cmd.arg("-o")
        .arg(&spv)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    cmd.spawn()?.wait_with_output()
}

fn list_shaders(dir: &Path) -> io::Result<Vec<PathBuf>> {
    fn collect_shaders_inner(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();

            if path.is_dir() {
                collect_shaders_inner(&path, out)?;
            } else if path.extension().is_some_and(|ext| ext == "slang") {
                out.push(path);
            }
        }

        Ok(())
    }

    let mut entries = vec![];
    collect_shaders_inner(dir, &mut entries)?;
    entries.sort();
    Ok(entries)
}

fn warn(message: impl AsRef<str>) {
    println!("cargo::warning={}", message.as_ref());
}

fn report_diagnostics(output: &Output) {
    for stream in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            if !line.trim().is_empty() {
                warn(format!("  {line}"));
            }
        }
    }
}
