use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct TestProject {
    pub path: PathBuf,
}

impl TestProject {
    /// Create a new temporary Cargo project in tests/fixtures/
    pub fn new(name: &str) -> Self {
        // Find workspace root - CARGO_MANIFEST_DIR is required
        let workspace_root: PathBuf = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR environment variable must be set. This is typically set automatically by Cargo when running tests.")
            .into();
        
        // let workspace_root = PathBuf::from(workspace_root)
        //     .canonicalize()
        //     .expect("Failed to canonicalize workspace root path");
        
        let project_path = workspace_root.join("tests/fixtures").join(name);
        
        // Clean up if it already exists
        if project_path.exists() {
            fs::remove_dir_all(&project_path).unwrap_or_else(|_| {
                panic!("Failed to remove existing test project: {:?}", project_path)
            });
        }
        
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(project_path.join("src")).unwrap();
        
        Self {
            path: project_path,
        }
    }

    /// Create a basic Cargo.toml (no async runtime)
    pub fn with_basic_cargo_toml(self) -> Self {
        let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(self.path.join("Cargo.toml"), cargo_toml).unwrap();
        self
    }

    /// Create a Cargo.toml with tokio
    pub fn with_tokio(self) -> Self {
        let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
"#;
        fs::write(self.path.join("Cargo.toml"), cargo_toml).unwrap();
        self
    }

    /// Create a Cargo.toml with async-std
    pub fn with_async_std(self) -> Self {
        let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
async-std = { version = "1.12", features = ["attributes"] }
"#;
        fs::write(self.path.join("Cargo.toml"), cargo_toml).unwrap();
        self
    }

    /// Create a Cargo.toml with smol
    pub fn with_smol(self) -> Self {
        let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
smol = "1.0"
"#;
        fs::write(self.path.join("Cargo.toml"), cargo_toml).unwrap();
        self
    }

    /// Create a minimal src/main.rs
    pub fn with_main_rs(self) -> Self {
        let main_rs = r#"fn main() {
    println!("Hello, world!");
}
"#;
        fs::write(self.path.join("src/main.rs"), main_rs).unwrap();
        self
    }

    /// Get the path to the generated __rsh.rs file
    pub fn rsh_bin_path(&self) -> PathBuf {
        self.path.join("src/bin/__rsh.rs")
    }

    /// Read the generated __rsh.rs file content
    pub fn read_rsh_bin(&self) -> String {
        fs::read_to_string(self.rsh_bin_path()).unwrap_or_default()
    }

    /// Fetch dependencies for this project (downloads to local cache)
    /// This allows tests to run offline after dependencies are fetched once
    pub fn fetch_dependencies(&self) -> bool {
        let output = Command::new("cargo")
            .arg("fetch")
            .arg("--manifest-path")
            .arg(self.path.join("Cargo.toml"))
            .output()
            .unwrap_or_else(|_| {
                // If fetch fails, try with current_dir (for older cargo versions)
                Command::new("cargo")
                    .arg("fetch")
                    .current_dir(&self.path)
                    .output()
                    .unwrap()
            });
        output.status.success()
    }

    /// Clean up the test project
    pub fn cleanup(&self) {
        if self.path.exists() {
            fs::remove_dir_all(&self.path).unwrap_or_else(|e| {
                eprintln!("Warning: Failed to cleanup test project {:?}: {}", self.path, e);
            });
        }
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        self.cleanup();
    }
}

