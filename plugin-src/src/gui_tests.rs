use super::*;
use std::fs;

fn input(name: &str) -> (PathBuf, PathBuf) {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "quality_ui_{name}_{}_{}",
        std::process::id(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    let source = dir.join("sample.ply");
    let mut text = String::from("ply\nformat ascii 1.0\nelement vertex 2\n");
    for key in [
        "x", "y", "z", "red", "green", "blue", "opacity", "scale_0", "scale_1", "scale_2", "rot_0",
        "rot_1", "rot_2", "rot_3",
    ] {
        text.push_str(&format!("property float {key}\n"));
    }
    text.push_str(
        "end_header\n0 0 0 100 120 180 -8 -2 -2 -2 1 0 0 0\n1 1 1 101 121 181 1 -2 -2 -2 1 0 0 0\n",
    );
    fs::write(&source, text).unwrap();
    (source, dir.join("result.su3ds"))
}

fn wait(app: &mut ConverterApp) {
    let until = Instant::now() + Duration::from_secs(5);
    while app.is_busy() && Instant::now() < until {
        app.poll_conversion();
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!app.is_busy(), "conversion did not complete");
    assert_eq!(app.status_kind, StatusKind::Complete, "{}", app.status_text);
}

#[test]
fn quality_ui_background_conversion_preserves_points() {
    let (source, output) = input("convert");
    let mut app = ConverterApp::headless();
    app.settings.auto_apply = false;
    app.settings.convert.quality = QualityPreset::Quality;
    app.set_paths(&source, &output);
    app.request_conversion();
    assert!(app.is_busy());
    wait(&mut app);
    let data = fs::read(&output).unwrap();
    assert_eq!(u32::from_le_bytes(data[8..12].try_into().unwrap()), 2);
    assert_eq!(u32::from_le_bytes(data[12..16].try_into().unwrap()), 2);
    assert_eq!(app.last_info().unwrap().points_written, 2);
    assert!(output.with_extension("json").exists());
    fs::remove_dir_all(source.parent().unwrap()).unwrap();
}

#[test]
fn overwrite_dialog_prevents_writing_until_confirmed() {
    let (source, output) = input("overwrite");
    fs::write(&output, b"do not overwrite yet").unwrap();
    let mut app = ConverterApp::headless();
    app.settings.auto_apply = false;
    app.set_paths(&source, &output);
    app.request_conversion();
    assert!(!app.is_busy());
    assert!(app.pending_overwrite.is_some());
    assert_eq!(fs::read(&output).unwrap(), b"do not overwrite yet");
    let pending = app.pending_overwrite.take().unwrap();
    app.begin_conversion(pending);
    wait(&mut app);
    assert_ne!(fs::read(&output).unwrap(), b"do not overwrite yet");
    fs::remove_dir_all(source.parent().unwrap()).unwrap();
}
