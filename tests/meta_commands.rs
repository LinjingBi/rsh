use rsh::rsh::{Session, Segment};

mod common;
use common::TestProject;

#[test]
fn test_reset_clears_buffers() {
    let project = TestProject::new("test_reset_clears_buffers")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        // Add some code to preamble and body
        session.add_code_block("use std::io;\nlet x = 5;");
        
        // Verify buffers are populated
        assert!(!session.preamble().is_empty());
        assert!(!session.body().is_empty());
        
        // Reset
        session.reset();
        
        // Verify buffers are cleared
        assert!(session.preamble().is_empty());
        assert!(session.body().is_empty());
    });
}

#[test]
fn test_reset_resets_mode() {
    let project = TestProject::new("test_reset_resets_mode")
        .with_tokio()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        // Add async code to trigger mode switch
        session.add_code_block("async fn test() {}\ntest().await;");
        let _ = session.run(); // This should switch to async mode
        
        // Verify mode is async
        assert!(matches!(session.mode(), rsh::rsh::Mode::Async(_)));
        
        // Reset
        session.reset();
        
        // Verify mode is back to Sync
        assert!(matches!(session.mode(), rsh::rsh::Mode::Sync));
    });
}

#[test]
fn test_reset_resets_prev_lengths() {
    let project = TestProject::new("test_reset_resets_prev_lengths")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        // Add some code
        session.add_code_block("let x = 5;");
        
        // Reset
        session.reset();
        
        // Access private fields through reflection is not possible, but we can verify
        // by checking that adding code again works correctly
        session.add_code_block("let y = 10;");
        session.add_code_block("let z = 15;");
        
        // Should have 2 body lines
        assert_eq!(session.body().len(), 2);
    });
}

#[test]
fn test_show_displays_empty_buffers() {
    let project = TestProject::new("test_show_empty")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let session = Session::new();
        
        // Capture stdout to verify show output
        // Note: We can't easily capture println! output in tests,
        // but we can verify the method doesn't panic
        session.show();
    });
}

#[test]
fn test_show_displays_populated_buffers() {
    let project = TestProject::new("test_show_populated")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("use std::io;\nstruct Test {}\nlet x = 5;\nprintln!(\"hello\");");
        
        // Verify buffers are populated
        assert_eq!(session.preamble().len(), 2);
        assert_eq!(session.body().len(), 2);
        
        // Show should not panic
        session.show();
    });
}

#[test]
fn test_delete_single_line_from_preamble() {
    let project = TestProject::new("test_delete_preamble_single")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("use std::io;\nuse std::fs;\nlet x = 5;");
        
        // Verify initial state
        assert_eq!(session.preamble().len(), 2);
        assert_eq!(session.body().len(), 1);
        
        // Delete first preamble line (index 0)
        session.delete(Segment::Preamble, &[0]);
        
        // Verify deletion
        assert_eq!(session.preamble().len(), 1);
        assert_eq!(session.preamble()[0], "use std::fs;");
        assert_eq!(session.body().len(), 1);
    });
}

#[test]
fn test_delete_single_line_from_body() {
    let project = TestProject::new("test_delete_body_single")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("let x = 5;\nlet y = 10;\nlet z = 15;");
        
        // Verify initial state
        assert_eq!(session.body().len(), 3);
        
        // Delete second body line (index 1)
        session.delete(Segment::Body, &[1]);
        
        // Verify deletion
        assert_eq!(session.body().len(), 2);
        assert_eq!(session.body()[0], "let x = 5;");
        assert_eq!(session.body()[1], "let z = 15;");
    });
}

#[test]
fn test_delete_multiple_lines_from_preamble() {
    let project = TestProject::new("test_delete_preamble_multiple")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("use std::io;\nuse std::fs;\nuse std::path;\nstruct Test {}");
        
        // Verify initial state
        assert_eq!(session.preamble().len(), 4);
        
        // Delete indices 0 and 2
        session.delete(Segment::Preamble, &[0, 2]);
        
        // Verify deletion (should delete from largest to smallest)
        assert_eq!(session.preamble().len(), 2);
        assert_eq!(session.preamble()[0], "use std::fs;");
        assert_eq!(session.preamble()[1], "struct Test {}");
    });
}

#[test]
fn test_delete_multiple_lines_from_body() {
    let project = TestProject::new("test_delete_body_multiple")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("let a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;\nlet e = 5;");
        
        // Verify initial state
        assert_eq!(session.body().len(), 5);
        
        // Delete indices 1, 3
        session.delete(Segment::Body, &[1, 3]);
        
        // Verify deletion
        assert_eq!(session.body().len(), 3);
        assert_eq!(session.body()[0], "let a = 1;");
        assert_eq!(session.body()[1], "let c = 3;");
        assert_eq!(session.body()[2], "let e = 5;");
    });
}

#[test]
fn test_delete_atomic_validation_invalid_index() {
    let project = TestProject::new("test_delete_atomic_invalid")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("let x = 5;\nlet y = 10;");
        
        // Verify initial state
        assert_eq!(session.body().len(), 2);
        
        // Try to delete invalid index (out of bounds)
        session.delete(Segment::Body, &[5]);
        
        // Verify nothing was deleted (atomic operation)
        assert_eq!(session.body().len(), 2);
        assert_eq!(session.body()[0], "let x = 5;");
        assert_eq!(session.body()[1], "let y = 10;");
    });
}

#[test]
fn test_delete_atomic_validation_mixed_valid_invalid() {
    let project = TestProject::new("test_delete_atomic_mixed")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("let a = 1;\nlet b = 2;\nlet c = 3;");
        
        // Verify initial state
        assert_eq!(session.body().len(), 3);
        
        // Try to delete valid (0) and invalid (10) indices
        session.delete(Segment::Body, &[0, 10]);
        
        // Verify nothing was deleted (atomic operation)
        assert_eq!(session.body().len(), 3);
        assert_eq!(session.body()[0], "let a = 1;");
        assert_eq!(session.body()[1], "let b = 2;");
        assert_eq!(session.body()[2], "let c = 3;");
    });
}

#[test]
fn test_delete_deduplicates_indices() {
    let project = TestProject::new("test_delete_dedup")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        session.add_code_block("let a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;");
        
        // Verify initial state
        assert_eq!(session.body().len(), 4);
        
        // Delete with duplicate indices
        session.delete(Segment::Body, &[1, 2, 1, 2]);
        
        // Verify only unique indices were deleted
        assert_eq!(session.body().len(), 2);
        assert_eq!(session.body()[0], "let a = 1;");
        assert_eq!(session.body()[1], "let d = 4;");
    });
}

#[test]
fn test_delete_from_empty_buffer() {
    let project = TestProject::new("test_delete_empty")
        .with_basic_cargo_toml()
        .with_main_rs();

    project.with_dir(|| {
        let mut session = Session::new();
        
        // Try to delete from empty body
        session.delete(Segment::Body, &[0]);
        
        // Should not panic, body should remain empty
        assert_eq!(session.body().len(), 0);
    });
}

