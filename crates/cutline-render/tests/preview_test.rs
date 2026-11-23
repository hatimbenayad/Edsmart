use cutline_core::project::Project;
use cutline_render::RenderPipeline;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_render_preview_frame_generation() {
    // REQ-PREVIEW-001: Frame capture to PNG
    let sample_project_path = PathBuf::from("examples/project.json");
    if !sample_project_path.exists() {
        return;
    }

    let content = fs::read_to_string(&sample_project_path).unwrap();
    let project: Project = serde_json::from_str(&content).unwrap();

    let out_png = std::env::temp_dir().join("cutline_test_preview_frame.png");
    let _ = fs::remove_file(&out_png);

    let info = RenderPipeline::render_preview_frame(&project, 15, &out_png).unwrap();

    assert_eq!(info.frame, 15);
    assert!(out_png.exists(), "PNG file must exist on disk");

    let bytes = fs::read(&out_png).unwrap();
    assert!(bytes.len() > 100, "PNG file must not be empty");
    // Verify PNG magic bytes
    assert_eq!(&bytes[0..4], &[0x89, b'P', b'N', b'G']);

    let _ = fs::remove_file(&out_png);
}
