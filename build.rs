//! Compila os `.slang` de `src/shaders/` para SPIR-V antes de cada build.
//!
//! Equivalente ao `slangc.sh`, mas plugado no `cargo build`: os shaders são
//! compilados em paralelo (um processo `slangc` por arquivo) e o relatório só
//! sai depois que todos terminam, na ordem em que foram encontrados.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;

const SHADER_DIR: &str = "src/shaders";
const ENTRY_POINTS: &[&str] = &["vertMain", "fragMain"];

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shader_dir = root.join(SHADER_DIR);

    // Rebuild quando um shader mudar (ou quando um novo for adicionado ao dir).
    // Os `.spv` gerados não estão nessa lista, então escrever dentro de `src/`
    // não dispara um novo build em loop.
    println!("cargo::rerun-if-changed={}", shader_dir.display());

    let shaders = collect_shaders(&shader_dir).expect("failed to collect shaders");

    for shader in &shaders {
        println!("cargo::rerun-if-changed={}", shader.display());
    }

    // Fan out: dispara todos os `slangc` de uma vez. A thread existe só para
    // drenar os pipes de stdout/stderr do filho — sem isso, um compilador
    // verboso encheria o buffer do pipe e travaria esperando alguém ler.
    let jobs: Vec<_> = shaders
        .into_iter()
        .map(|shader| thread::spawn(move || (compile(&shader), shader)))
        .collect();

    // Await: `join` bloqueia até aquele job terminar e devolve o resultado
    // dele. Como o loop percorre na ordem original, o relatório é determinístico
    // mesmo com os processos terminando fora de ordem.
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

/// Cargo engole stdout/stderr de build script (só aparecem com `-vv`), então a
/// única forma de falar com o usuário durante um build bem-sucedido é
/// `cargo::warning=`. Uma linha por mensagem: `\n` embutido é truncado.
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
