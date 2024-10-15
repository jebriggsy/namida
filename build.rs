use std::{
    process::Command,
    env,
};

fn main() {
    // Check if we are in a Nix environment
    // Nix does not work well with with variable compile-time outputs
    // Git revision will be supplied correctly by the Nix build environment
    // Compile DT will be the start of the UTC epoch
    match env::var("NIX_BUILD_TOP"){
        Ok(..) => println!("Nix builder detected, disabling non-deterministic compile time operations"),
        Err(..) => {
            // https://stackoverflow.com/a/44407625
            let output = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap();
            let git_hash = String::from_utf8(output.stdout).unwrap();
            println!("cargo:rustc-env=GIT_HASH={}", git_hash);

            // Get compilation date / time
            let dt_local = chrono::Local::now();
            let naive_utc = dt_local.naive_utc();
            let formatted = naive_utc.format("%Y-%m-%d %H:%M:%S");
            println!("cargo:rustc-env=NAMIDA_COMPILE_DT={} UTC", formatted);
        },
    };
}
