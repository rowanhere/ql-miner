use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if env::var_os("CARGO_FEATURE_CUDA").is_none() {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let object = out_dir.join("cuda_miner.o");
    let cuda_arch = env::var("CUDA_ARCH").unwrap_or_else(|_| "sm_89".to_string());

    let status = Command::new("nvcc")
        .arg("-O3")
        .arg("-arch")
        .arg(&cuda_arch)
        .arg("-Xcompiler")
        .arg("-fPIC")
        .arg("-c")
        .arg("src/cuda_miner.cu")
        .arg("-o")
        .arg(&object)
        .status()
        .expect("failed to run nvcc; install CUDA toolkit or build without --features cuda");

    if !status.success() {
        panic!("nvcc failed while compiling src/cuda_miner.cu");
    }

    println!("cargo:rustc-link-arg={}", object.display());
    if let Ok(cuda_home) = env::var("CUDA_HOME").or_else(|_| env::var("CUDA_PATH")) {
        println!("cargo:rustc-link-search=native={cuda_home}/lib64");
    } else {
        println!("cargo:rustc-link-search=native=/usr/local/cuda/lib64");
    }
    println!("cargo:rustc-link-lib=cudart");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=gcc_s");
    }
    println!("cargo:rerun-if-changed=src/cuda_miner.cu");
}
