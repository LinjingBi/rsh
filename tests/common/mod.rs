use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct TestProject {
    pub path: PathBuf,
}

impl TestProject {
    /// Create a new temporary Cargo project in tests/fixtures/
    pub fn new(name: &str) -> Self {
        let project_path = Path::new("tests/fixtures").join(name);
        
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

    /// Verify that __rsh.rs exists
    pub fn rsh_bin_exists(&self) -> bool {
        self.rsh_bin_path().exists()
    }

    /// Change to this project's directory and run a closure
    pub fn with_dir<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&self.path).unwrap();
        let result = f();
        std::env::set_current_dir(original_dir).unwrap();
        result
    }

    /// Clean up the test project
    pub fn cleanup(&self) {
        if self.path.exists() {
            fs::remove_dir_all(&self.path).unwrap_or_else(|e| {
                eprintln!("Warning: Failed to cleanup test project {:?}: {}", self.path, e);
            });
        }
    }

    /// Run cargo build to ensure the project is valid
    pub fn build(&self) -> bool {
        self.with_dir(|| {
            let output = Command::new("cargo")
                .arg("build")
                .arg("--quiet")
                .output()
                .unwrap();
            output.status.success()
        })
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        self.cleanup();
    }
}

