use std::env;
use std::path::PathBuf;
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

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("output dir"));
    let resource_file = output_dir.join("autoclicker.res");
    let resource_script = manifest_dir.join("resources").join("autoclicker.rc");
    let resource_compiler = env::var_os("RC").unwrap_or_else(|| "rc.exe".into());

    let result = Command::new(resource_compiler)
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
        Ok(status) => panic!("Windows resource compiler failed with status {status}"),
        Err(error) => panic!("Could not run rc.exe to embed the application icon: {error}"),
    }
}
