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
fn template_limit_payload_applies_zero_without_overwriting_other_properties() {
    let mut settings = Settings::default();
    for template in [QualityTemplate::Preview, QualityTemplate::Balanced] {
        settings.apply_quality_template(template);
        let props =
            conversion_properties(Path::new("scene.su3ds"), conversion_apply_limit(&settings));
        assert_eq!(
            props,
            vec![
                (FILE_ITEM.to_string(), "scene.su3ds".to_string()),
                (RENDER_LIMIT.to_string(), "0".to_string()),
            ]
        );
    }

    settings.convert.dither = false;
    assert_eq!(conversion_apply_limit(&settings), None);
    settings.convert.quality = QualityPreset::Quality;
    settings.convert.keep_sh = false;
    assert_eq!(conversion_apply_limit(&settings), Some(0));
}

#[test]
fn conversion_captures_template_limit_and_auto_apply_off_does_not_apply_it() {
    let (source, output) = input("limit_snapshot");
    let mut app = ConverterApp::headless();
    app.settings.auto_apply = false;
    app.settings
        .apply_quality_template(QualityTemplate::Balanced);
    app.set_paths(&source, &output);
    app.request_conversion();

    assert_eq!(app.job.as_ref().unwrap().apply_limit, Some(0));
    app.settings.render.limit = 12_345;
    app.settings.convert.quality = QualityPreset::Quality;
    wait(&mut app);

    assert_eq!(app.last_apply_limit, Some(0));
    assert_eq!(app.last_applied(), 0);
    assert_eq!(app.settings.render.limit, 12_345);
    fs::remove_dir_all(source.parent().unwrap()).unwrap();
}

#[test]
fn manually_apply_last_uses_the_limit_captured_by_that_conversion() {
    let (source, output) = input("last_limit");
    let mut app = ConverterApp::headless();
    app.settings.auto_apply = false;
    app.settings
        .apply_quality_template(QualityTemplate::Preview);
    app.set_paths(&source, &output);
    app.request_conversion();
    wait(&mut app);
    assert_eq!(app.last_apply_limit, Some(0));

    let props = conversion_properties(&output, app.last_conversion_limit(&output));
    assert_eq!(props[1], (RENDER_LIMIT.to_string(), "0".to_string()));
    // 既存ファイルBを適用して直前記録がBになっても、Aの上限を流用しない。
    let existing = output.with_file_name("existing.su3ds");
    fs::copy(&output, &existing).unwrap();
    assert_eq!(app.last_conversion_limit(&existing), None);
    let props = conversion_properties(&existing, app.last_conversion_limit(&existing));
    assert_eq!(props.len(), 1);
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

/// 進捗スロットへ任意の値を入れ、その状態の描画テキストを取り出す。
fn draw_with_progress(app: &mut ConverterApp, progress: Option<ConvertProgress>) -> String {
    let context = egui::Context::default();
    let mut output = None;
    for _ in 0..2 {
        output = Some(context.run_ui(egui::RawInput::default(), |ui| {
            if let Some(job) = &app.job
                && let Ok(mut slot) = job.progress.lock()
            {
                *slot = progress;
            }
            app.draw(ui);
        }));
    }
    let output = output.unwrap();
    let mut text = String::new();
    collect_text(&output.shapes, &mut text);
    text
}

fn collect_text(shapes: &[egui::epaint::ClippedShape], text: &mut String) {
    fn walk(shape: &egui::Shape, text: &mut String) {
        match shape {
            egui::Shape::Text(value) => {
                text.push_str(&value.galley.job.text);
                text.push('\n');
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, text);
                }
            }
            _ => {}
        }
    }
    for clipped in shapes {
        walk(&clipped.shape, text);
    }
}

/// 変換中ジョブを持たせ、進捗表示と中止ボタンが描画されることを確かめる。
fn busy_app() -> ConverterApp {
    let (source, output) = input("progress");
    let mut app = ConverterApp::headless();
    app.settings.convert.quality = QualityPreset::Quality;
    app.settings.convert.all_points = true;
    let _ = (source, output);
    // 変換は数ミリ秒で終わるため、実ジョブでは描画前に完了してしまう。
    // 進捗表示の検証には、結果を返さない停止ジョブを使う。
    let (sender, receiver) = mpsc::channel();
    std::mem::forget(sender); // 送信されないまま接続を保つ
    app.job = Some(ConversionJob {
        receiver,
        progress: Arc::new(Mutex::new(None)),
        cancel: CancelFlag::new(),
        handle: None,
        targets: Vec::new(),
        epoch: 0,
        output: PathBuf::from("result.su3ds"),
        quality: true,
        auto_apply: false,
        apply_limit: None,
    });
    app
}

#[test]
fn processing_progress_shows_percentage_and_points() {
    let mut app = busy_app();
    let text = draw_with_progress(
        &mut app,
        Some(ConvertProgress {
            stage: ConvertStage::Processing,
            fraction: Some(0.42),
            points_done: 4200,
            points_total: 10000,
        }),
    );
    assert!(text.contains("変換 42%"), "{text}");
    assert!(text.contains("4,200 / 10,000 点"), "{text}");
    assert!(
        text.contains("中止"),
        "中止ボタンが描画されていません: {text}"
    );
}

#[test]
fn loading_progress_shows_indeterminate_stage() {
    let mut app = busy_app();
    let text = draw_with_progress(
        &mut app,
        Some(ConvertProgress {
            stage: ConvertStage::Loading,
            fraction: None,
            points_done: 0,
            points_total: 0,
        }),
    );
    assert!(text.contains("読み込み"), "{text}");
    assert!(text.contains("中止"), "{text}");
}

#[test]
fn missing_progress_shows_preparing_message() {
    let mut app = busy_app();
    let text = draw_with_progress(&mut app, None);
    assert!(text.contains("準備しています"), "{text}");
}

#[test]
fn narrow_panel_keeps_progress_and_abort_visible() {
    let mut app = busy_app();
    let context = egui::Context::default();
    let mut output = None;
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(300.0, 480.0),
            )),
            ..Default::default()
        };
        output = Some(context.run_ui(input, |ui| {
            if let Some(job) = &app.job
                && let Ok(mut slot) = job.progress.lock()
            {
                *slot = Some(ConvertProgress {
                    stage: ConvertStage::Processing,
                    fraction: Some(0.5),
                    points_done: 50,
                    points_total: 100,
                });
            }
            app.draw(ui);
        }));
    }
    let output = output.unwrap();
    let mut text = String::new();
    collect_text(&output.shapes, &mut text);
    assert!(text.contains("変換 50%"), "{text}");
    assert!(text.contains("中止"), "{text}");
    // 画面外へはみ出す描画がないこと。
    let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 480.0));
    let mut outside = Vec::new();
    collect_outside(&output.shapes, bounds, &mut outside);
    assert!(outside.is_empty(), "画面外の描画: {outside:?}");
}

fn collect_outside(
    shapes: &[egui::epaint::ClippedShape],
    bounds: egui::Rect,
    found: &mut Vec<String>,
) {
    fn walk(shape: &egui::Shape, bounds: egui::Rect, found: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => {
                if !text.galley.job.text.is_empty()
                    && !bounds.contains_rect(text.visual_bounding_rect())
                {
                    found.push(text.galley.job.text.clone());
                }
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, bounds, found);
                }
            }
            _ => {}
        }
    }
    for clipped in shapes {
        walk(&clipped.shape, bounds, found);
    }
}
