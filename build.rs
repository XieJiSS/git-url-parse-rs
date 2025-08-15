fn main() {
    // https://pyo3.rs/v0.25.1/building-and-distribution.html
    pyo3_build_config::add_extension_module_link_args();

    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        // only run this when which::which shows that we are using system python3
        if let Ok(python_executable) = which::which("python3") {
            if python_executable.starts_with("/usr/bin/") {
                pyo3_build_config::add_python_framework_link_args();
            }
        }
    }
}
