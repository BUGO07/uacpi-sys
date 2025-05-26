use std::process::Command;
use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

const SOURCES: &[&str] = &[
    "source/default_handlers.c",
    "source/event.c",
    "source/interpreter.c",
    "source/io.c",
    "source/mutex.c",
    "source/namespace.c",
    "source/notify.c",
    "source/opcodes.c",
    "source/opregion.c",
    "source/osi.c",
    "source/registers.c",
    "source/resources.c",
    "source/shareable.c",
    "source/sleep.c",
    "source/stdlib.c",
    "source/tables.c",
    "source/types.c",
    "source/uacpi.c",
    "source/utilities.c",
];

fn init_submodule(uacpi_path: &Path) {
    if !uacpi_path.join("README.md").exists() {
        Command::new("git")
            .args(["submodule", "update", "--init"])
            .current_dir(uacpi_path)
            .status()
            .expect("failed to retrieve uACPI sources with git");
    } else {
        Command::new("git")
            .args(["submodule", "update", "--remote"])
            .current_dir(uacpi_path)
            .status()
            .expect("failed to retrieve uACPI sources with git");
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let project_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let uacpi_path = Path::new(&project_dir).join("uACPI");

    init_submodule(&uacpi_path);

    let uacpi_path_str = uacpi_path.to_str().unwrap();

    let sources = SOURCES
        .iter()
        .map(|file| format!("{uacpi_path_str}/{file}"));

    let mut cc = cc::Build::new();

    cc.compiler("clang")
        .files(sources)
        .include(format!("{uacpi_path_str}/include"))
        .define("UACPI_SIZED_FREES", "1")
        .flag("-nostdlib")
        .flag("-ffreestanding")
        .flag("-fno-stack-protector")
        .flag("-fno-PIC")
        .flag("-fno-PIE");

    let target = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if target.contains("x86_64") || target.contains("i686") {
        cc.flag("-mno-red-zone").flag("-mcmodel=kernel");
    }

    if !target.contains("riscv64") {
        cc.flag("-mgeneral-regs-only");
    }

    if cfg!(feature = "reduced-hardware") {
        cc.define("UACPI_REDUCED_HARDWARE", "1");
    }

    if cfg!(feature = "barebones-mode") {
        cc.define("UACPI_BAREBONES_MODE", "1");
    }

    cc.compile("uacpi");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_args(&[
            "-IuACPI/include",
            "-DUACPI_SIZED_FREES=1",
            #[cfg(feature = "reduced-hardware")]
            "-DUACPI_REDUCED_HARDWARE=1",
            #[cfg(feature = "barebones-mode")]
            "-DUACPI_BAREBONES_MODE=1",
            "-ffreestanding",
        ])
        .prepend_enum_name(false)
        .use_core()
        .derive_default(true)
        .derive_debug(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    Ok(())
}
