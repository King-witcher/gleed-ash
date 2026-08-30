//! Compiles the `.slang` files in `src/shaders/` to SPIR-V before every build.
//!
//! Equivalent to `slangc.sh`, but plugged into `cargo build`: the shaders
//! compile in parallel (one `slangc` process per file) and the report only
//! comes out after all of them finish, in the order they were found.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;

const SHADER_DIR: &str = "src/shaders";
const ENTRY_POINTS: &'static [&'static str] = &["vertMain", "fragMain"];

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shader_dir = root.join(SHADER_DIR);

    // Rebuild when a shader changes (or when a new one is added to the dir).
    // The generated `.spv` files are not on this list, so writing inside
    // `src/` does not trigger another build in a loop.
    println!("cargo::rerun-if-changed={}", shader_dir.display());

    let shaders = collect_shaders(&shader_dir).expect("failed to collect shaders");

    for shader in &shaders {
        println!("cargo::rerun-if-changed={}", shader.display());
    }

    // Fan out: spawns every `slangc` at once. The thread exists only to drain
    // the child's stdout/stderr pipes — without it, a verbose compiler would
    // fill the pipe buffer and stall waiting for a reader.
    let jobs: Vec<_> = shaders
        .into_iter()
        .map(|shader| thread::spawn(move || (compile(&shader), shader)))
        .collect();

    // Await: `join` blocks until that job finishes and returns its result.
    // Since the loop walks in the original order, the report is deterministic
    // even with the processes finishing out of order.
    let total = jobs.len();
    let mut failures = 0;

    for job in jobs {
        let (result, shader) = job.join().expect("thread de compilação entrou em panic");
        let shader = shader.strip_prefix(&root).unwrap_or(&shader).display();

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
                warn(format!(
                    "[FAILED] {shader}: não foi possível executar slangc: {e}"
                ));
            }
        }
    }

    if failures > 0 {
        panic!("slangc: {failures} de {total} shader(s) falharam");
    }

    warn(format!("slangc: {total} shader(s) compilado(s)"));
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

fn collect_shaders(dir: &Path) -> io::Result<Vec<PathBuf>> {
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

/// Cargo swallows a build script's stdout/stderr (they only show with `-vv`),
/// so the only way to talk to the user during a successful build is
/// `cargo::warning=`. One line per message: embedded `\n` gets truncated.
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
