use rsh::rsh::{Session, Mode, AsyncRuntime};
use std::fs;

mod common;
use common::TestProject;

#[test]
fn test_sync_mode_code_executes_successfully() {
    let project = TestProject::new("test_sync_success")
    .with_basic_cargo_toml()
    .with_main_rs();

    let mut session = Session::new(Some(&project.path));
    
    // Verify starts in sync mode
    assert!(matches!(session.mode(), Mode::Sync));
    
    // Add simple sync code
    session.add_code_block("let x = 5;\nprintln!(\"x = {}\", x);");
    
    // Run should succeed
    let result = session.run();
    assert!(result.is_ok());
    
    // Should still be in sync mode
    assert!(matches!(session.mode(), Mode::Sync));
    
    // Verify generated file has sync structure
    let generated = project.read_rsh_bin();
    assert!(generated.contains("fn __rsh_session()"));
    assert!(!generated.contains("async fn __rsh_session()"));
    assert!(generated.contains("fn main()"));
}

#[test]
fn test_async_code_triggers_switch_with_tokio() {
    let project = TestProject::new("test_async_tokio")
    .with_tokio()
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();

    let mut session = Session::new(Some(&project.path));
    
    // Verify starts in sync mode
    assert!(matches!(session.mode(), Mode::Sync));
    
    // Add async code that will fail in sync mode
    session.add_code_block("async fn test() {}\ntest().await;");
    
    // Run - should detect async error and switch
    let result = session.run();
    // run() always returns Ok, but may have printed errors
    assert!(result.is_ok());
    
    // Should have switched to async mode with tokio
    assert!(matches!(session.mode(), Mode::Async(AsyncRuntime::Tokio)));
    
    // Verify generated file has async structure with tokio
    let generated = project.read_rsh_bin();
    assert!(generated.contains("async fn __rsh_session()"));
    assert!(generated.contains("#[tokio::main]"));
    assert!(generated.contains("async fn main()"));
    assert!(generated.contains(".await"));
}

#[test]
fn test_async_code_triggers_switch_with_async_std() {
    let project = TestProject::new("test_async_async_std")
    .with_async_std()
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();

    let mut session = Session::new(Some(&project.path));
    
    // Verify starts in sync mode
    assert!(matches!(session.mode(), Mode::Sync));
    
    // Add async code
    session.add_code_block("async fn test() {}\ntest().await;");
    
    // Run - should detect async error and switch
    let _ = session.run();
    
    // Should have switched to async mode with async-std
    assert!(matches!(session.mode(), Mode::Async(AsyncRuntime::AsyncStd)));
    
    // Verify generated file has async structure with async-std
    let generated = project.read_rsh_bin();
    assert!(generated.contains("async fn __rsh_session()"));
    assert!(generated.contains("#[async_std::main]"));
    assert!(generated.contains("async fn main()"));
}

#[test]
fn test_async_code_triggers_switch_with_smol() {
    let project = TestProject::new("test_async_smol")
    .with_smol()
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();

    let mut session = Session::new(Some(&project.path));
    
    // Verify starts in sync mode
    assert!(matches!(session.mode(), Mode::Sync));
    
    // Add async code
    session.add_code_block("async fn test() {}\ntest().await;");
    
    // Run - should detect async error and switch
    let _ = session.run();
    
    // Should have switched to async mode with smol
    assert!(matches!(session.mode(), Mode::Async(AsyncRuntime::Smol)));
    
    // Verify generated file has async structure with smol
    let generated = project.read_rsh_bin();
    assert!(generated.contains("async fn __rsh_session()"));
    assert!(generated.contains("smol::block_on"));
    assert!(generated.contains("fn main()"));
    assert!(!generated.contains("#[tokio::main]"));
    assert!(!generated.contains("#[async_std::main]"));
}

#[test]
fn test_runtime_preference_tokio_over_async_std() {
    let project = TestProject::new("test_runtime_preference")
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();

    // Create Cargo.toml with both tokio and async-std (tokio should be preferred)
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
async-std = { version = "1.12", features = ["attributes"] }
"#;
    let cargo_path = project.path.join("Cargo.toml");
    fs::write(&cargo_path, cargo_toml).unwrap();

    // Fetch dependencies before testing
    project.fetch_dependencies();

    let mut session = Session::new(Some(&project.path));
    
    // Add async code
    session.add_code_block("async fn test() {}\ntest().await;");
    
    // Run - should switch to tokio (preferred)
    let _ = session.run();
    
    // Should have switched to tokio, not async-std
    assert!(matches!(session.mode(), Mode::Async(AsyncRuntime::Tokio)));
}

#[test]
fn test_async_code_fails_gracefully_no_runtime() {
    let project = TestProject::new("test_async_no_runtime")
    .with_basic_cargo_toml()
    .with_main_rs();

    let mut session = Session::new(Some(&project.path));
        
    // Verify starts in sync mode
    assert!(matches!(session.mode(), Mode::Sync));
        
    // Add async code
    session.add_code_block("async fn test() {}\ntest().await;");
        
    // Run - should detect async error but no runtime available
    let _ = session.run();
        
    // Should still be in sync mode (no runtime found)
    assert!(matches!(session.mode(), Mode::Sync));
        
        // Code should have been rolled back (failed, not async error)
        // The buffers should be rolled back to previous state
        // Since we started empty, they should be empty
    assert_eq!(session.preamble().len(), 0);
    assert_eq!(session.body().len(), 0);

}

#[test]
fn test_mode_persistence_once_async() {
    let project = TestProject::new("test_mode_persistence")
    .with_tokio()
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();


    let mut session = Session::new(Some(&project.path));
        
        // Switch to async mode first
    session.add_code_block("async fn test() {}\ntest().await;");
    let _ = session.run();
    assert!(matches!(session.mode(), Mode::Async(AsyncRuntime::Tokio)));
        
    // Add more async code
    session.add_code_block("async fn test2() {}\ntest2().await;");
    let _ = session.run();
        
    // Should still be in async mode (not switch again)
    assert!(matches!(session.mode(), Mode::Async(AsyncRuntime::Tokio)));

}

#[test]
fn test_successful_sync_code_no_switch() {
    let project = TestProject::new("test_sync_no_switch")
    .with_tokio()
    .with_main_rs();

    let mut session = Session::new(Some(&project.path));
        
    // Add sync code that succeeds
    session.add_code_block("let x = 42;\nprintln!(\"x = {}\", x);");
    let _ = session.run();
        
    // Should still be in sync mode
    assert!(matches!(session.mode(), Mode::Sync));

}

#[test]
fn test_non_async_errors_dont_trigger_switch() {
    let project = TestProject::new("test_non_async_error")
    .with_tokio()
    .with_main_rs();

    let mut session = Session::new(Some(&project.path));
        
    // Add code with a non-async error (e.g., type error)
    session.add_code_block("let x: i32 = \"not a number\";");
    let _ = session.run();
        
    // Should still be in sync mode (not an async error)
    assert!(matches!(session.mode(), Mode::Sync));
        
        // Code should have been rolled back
    assert_eq!(session.preamble().len(), 0);
    assert_eq!(session.body().len(), 0);

}

#[test]
fn test_generated_code_structure_sync() {
    let project = TestProject::new("test_gen_sync")
    .with_basic_cargo_toml()
    .with_main_rs();

    let mut session = Session::new(Some(&project.path));
        
    session.add_code_block("use std::io;\nstruct Test {}\nlet x = 5;\nprintln!(\"test\");");
    let _ = session.run();
        
    let generated = project.read_rsh_bin();
        
        // Check preamble is at module scope
    assert!(generated.contains("use std::io;"));
    assert!(generated.contains("struct Test {}"));
        
        // Check body is in function
    assert!(generated.contains("fn __rsh_session()"));
    assert!(!generated.contains("async fn __rsh_session()"));
    assert!(generated.contains("    let x = 5;"));
    assert!(generated.contains("    println!(\"test\");"));
        
        // Check main function
    assert!(generated.contains("fn main()"));
    assert!(generated.contains("__rsh_session()"));

}

#[test]
fn test_generated_code_structure_async_tokio() {
    let project = TestProject::new("test_gen_async_tokio")
    .with_tokio()
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();


    let mut session = Session::new(Some(&project.path));
        
    session.add_code_block("use std::io;\nstruct Test {}\nasync fn test() {}\ntest().await;");
    let _ = session.run();
        
    let generated = project.read_rsh_bin();
        
        // Check preamble is at module scope
    assert!(generated.contains("use std::io;"));
    assert!(generated.contains("struct Test {}"));
        
        // Check async function
    assert!(generated.contains("async fn __rsh_session()"));
    assert!(generated.contains("    async fn test() {}"));
    assert!(generated.contains("    test().await;"));
        
        // Check tokio main
    assert!(generated.contains("#[tokio::main]"));
    assert!(generated.contains("async fn main()"));
    assert!(generated.contains("__rsh_session().await"));

}

#[test]
fn test_generated_code_structure_async_smol() {
    let project = TestProject::new("test_gen_async_smol")
    .with_smol()
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();

    let mut session = Session::new(Some(&project.path));
        
    session.add_code_block("async fn test() {}\ntest().await;");
    let _ = session.run();
        
    let generated = project.read_rsh_bin();
        
        // Check async function
    assert!(generated.contains("async fn __rsh_session()"));
        
        // Check smol main structure
    assert!(generated.contains("fn main()"));
    assert!(generated.contains("smol::block_on"));
    assert!(generated.contains("__rsh_session().await"));
    assert!(!generated.contains("#[tokio::main]"));
    assert!(!generated.contains("#[async_std::main]"));

}

#[test]
fn test_preamble_and_body_placement_async() {
    // has async crate but code block does not have async syntax
    // so the session and generated fn shoud be sync

    let project = TestProject::new("test_preamble_body_async")
    .with_tokio()
    .with_main_rs();

    // Fetch dependencies before testing
    project.fetch_dependencies();


    let mut session = Session::new(Some(&project.path));
        
    // Add preamble items and body items
    session.add_code_block("use std::fs;\nstruct MyStruct {}\nlet x = 10;\nprintln!(\"x = {}\", x);");
    let _ = session.run();
        
    let generated = project.read_rsh_bin();
        
    // Preamble should be before sync function
    let preamble_pos = generated.find("use std::fs;")
    .expect("Generated code should contain 'use std::fs;'");
    assert!(!generated.contains("async fn __rsh_session()"));
    let async_fn_pos = generated.find("fn __rsh_session()")
    .expect("Generated code should contain 'fn __rsh_session()'");
    assert!(preamble_pos < async_fn_pos);

    // Body should be inside sync function
    assert!(generated.contains("    let x = 10;"));
    assert!(generated.contains("    println!(\"x = {}\", x);"));

}

