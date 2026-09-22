fn main() {
    for key in [
        "ZERO_MOD_MANAGER_PROJECT_URL",
        "ZERO_MOD_MANAGER_RELEASE_API",
        "ZERO_MOD_MANAGER_NEXUS_URL",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    for (key, fallback) in [
        (
            "ZERO_MOD_MANAGER_PROJECT_URL",
            "https://github.com/stellamarislabs/zero-mod-manager",
        ),
        (
            "ZERO_MOD_MANAGER_RELEASE_API",
            "https://api.github.com/repos/stellamarislabs/zero-mod-manager/releases/latest",
        ),
    ] {
        let value = std::env::var(key).unwrap_or_else(|_| fallback.to_string());
        println!("cargo:rustc-env={key}={value}");
    }
    tauri_build::build()
}
