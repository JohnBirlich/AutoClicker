use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=resources/autoclicker.rc");
    println!("cargo:rerun-if-changed=assets/autoclicker.ico");
    println!("cargo:rerun-if-env-changed=RC");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }

    let Some(resource_compiler) = find_resource_compiler() else {
        println!(
            "cargo:warning=rc.exe was not found; the app will use the adjacent .ico file at runtime"
        );
        return;
    };

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("output dir"));
    let resource_file = output_dir.join("autoclicker.res");
    let resource_script = manifest_dir.join("resources").join("autoclicker.rc");

    let result = Command::new(&resource_compiler)
        .current_dir(&manifest_dir)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource_file)
        .arg(&resource_script)
        .status();

    match result {
        Ok(status) if status.success() => {
            println!(
                "cargo:rustc-link-arg-bin=autoclicker={}",
                resource_file.display()
            );
        }
        Ok(status) => println!(
            "cargo:warning=rc.exe exited with {status}; the app will use the adjacent .ico file at runtime"
        ),
        Err(error) => println!(
            "cargo:warning=Could not run {} ({error}); the app will use the adjacent .ico file at runtime",
            Path::new(&resource_compiler).display()
        ),
    }
}

fn find_resource_compiler() -> Option<OsString> {
    if let Some(compiler) = env::var_os("RC") {
        if !compiler.is_empty() {
            return Some(compiler);
        }
    }

    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            let candidate = directory.join("rc.exe");
            if candidate.is_file() {
                return Some(candidate.into_os_string());
            }
        }
    }

    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        let Some(program_files) = env::var_os(variable) else {
            continue;
        };
        if let Some(candidate) = find_in_windows_kit(Path::new(&program_files)) {
            return Some(candidate.into_os_string());
        }
    }

    None
}

fn find_in_windows_kit(program_files: &Path) -> Option<PathBuf> {
    let kit_bin = program_files.join("Windows Kits").join("10").join("bin");
    let mut version_dirs: Vec<PathBuf> = std::fs::read_dir(kit_bin)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    version_dirs.sort_by(|left, right| right.file_name().cmp(&left.file_name()));

    version_dirs
        .into_iter()
        .map(|version| version.join("x64").join("rc.exe"))
        .find(|candidate| candidate.is_file())
}
