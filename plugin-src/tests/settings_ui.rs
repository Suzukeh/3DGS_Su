//! 設定モデルとUIの境界テスト。
//!
//! 公開APIだけを使う統合テスト。GUIのホストAPIはヘッドレス構築では呼ばれないため、
//! AviUtl2を起動せずに検証できる。

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use aviutl2_eframe::egui;
use three_dgs_converter_su::gui::{ConverterApp, existing_outputs, normalized_su3ds_path};
use three_dgs_converter_su::settings::{
    BlendMode, ConvertSettings, FitMode, QualityPreset, QualityTemplate, RENDER_ALPHA,
    RENDER_BLEND, RENDER_LIMIT, RENDER_SIZE, RenderSettings, Settings, format_number,
};

fn temp_dir(name: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("3dgs_ui_{name}_{}_{}", std::process::id(), serial));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn defaults_match_the_contract() {
    let settings = Settings::default();
    assert!(settings.auto_apply);
    assert_eq!(settings.convert.quality, QualityPreset::Lightweight);
    assert_eq!(settings.convert.max_points, 30_000);
    assert!(!settings.convert.all_points);
    assert_eq!(settings.convert.fit_mode, FitMode::Normalized);
    assert_eq!(settings.convert.fit_size, 400.0);
    assert!(settings.convert.center);
    assert!(settings.convert.dither);
    assert!(settings.convert.keep_sh);
    assert_eq!(settings.render.limit, 30_000);
    assert_eq!(settings.render.blend, BlendMode::Normal);

    let options = settings.convert.to_convert_options();
    assert_eq!(options.max, 30_000);
    assert!(!options.preserve_quality);
    assert!(options.keep_sh);
    assert_eq!(options.fit, 400.0);
    assert!(!options.no_center);
    assert!(!options.no_dither);
}

#[test]
fn quality_preserves_precision_and_honors_explicit_sh_setting() {
    let mut convert = ConvertSettings {
        quality: QualityPreset::Quality,
        all_points: false,
        max_points: 123,
        fit_mode: FitMode::Original,
        center: false,
        flip_y: true,
        flip_z: true,
        dither: false,
        keep_sh: false,
        ..ConvertSettings::default()
    };
    let options = convert.to_convert_options();
    assert_eq!(options.max, 0);
    assert!(options.preserve_quality);
    assert!(options.no_dither);
    assert!(!options.keep_sh);
    assert_eq!(options.fit, 0.0);
    assert!(options.no_center);
    assert!(options.flip_y);
    assert!(options.flip_z);

    convert.quality = QualityPreset::Lightweight;
    let options = convert.to_convert_options();
    assert_eq!(options.max, 123);
    assert!(!options.preserve_quality);
    assert!(!options.keep_sh);
    assert!(options.no_dither);
}

#[test]
fn all_points_switch_and_custom_limit_are_independent() {
    let mut convert = ConvertSettings {
        all_points: true,
        max_points: 1_234_567,
        ..Default::default()
    };
    assert_eq!(convert.to_convert_options().max, 0);
    assert!(convert.uses_all_points());

    convert.all_points = false;
    assert_eq!(convert.to_convert_options().max, 1_234_567);
    assert!(!convert.uses_all_points());

    convert.max_points = 0;
    assert_eq!(convert.to_convert_options().max, 0);
    assert!(convert.uses_all_points());
}

#[test]
fn render_properties_use_japanese_names_and_blend_labels() {
    let render = RenderSettings {
        size: 1.5,
        alpha: 0.25,
        near: 3.0,
        limit: 0,
        sort: false,
        ellipse: true,
        panorama: true,
        fov: 90.0,
        blend: BlendMode::Additive,
    };
    let properties = render.to_properties();
    assert!(properties.contains(&(RENDER_SIZE.into(), "1.5".into())));
    assert!(properties.contains(&(RENDER_ALPHA.into(), "0.25".into())));
    assert!(properties.contains(&(RENDER_LIMIT.into(), "0".into())));
    assert!(properties.contains(&(RENDER_BLEND.into(), "加算".into())));
    assert!(properties.contains(&("奥から描く".into(), "0".into())));
    assert!(properties.contains(&("パノラマ".into(), "1".into())));

    let mut loaded = RenderSettings::default();
    let count = loaded.merge_properties([
        ("size", "2.5"),
        ("不透明度", "0.5"),
        ("表示上限", "0"),
        ("合成", "通常"),
        ("奥から描く", "0"),
        ("パノラマ", "1"),
        ("未知", "1"),
    ]);
    assert_eq!(count, 6);
    assert_eq!(loaded.size, 2.5);
    assert_eq!(loaded.alpha, 0.5);
    assert_eq!(loaded.limit, 0);
    assert_eq!(loaded.blend, BlendMode::Normal);
    assert!(!loaded.sort);
    assert!(loaded.panorama);
}

#[test]
fn render_limits_accept_zero_and_float_strings() {
    let mut render = RenderSettings::default();
    assert!(render.set_property("limit", "0"));
    assert_eq!(render.limit, 0);
    assert!(render.set_property("表示上限", "1234.0"));
    assert_eq!(render.limit, 1234);
    assert!(!render.set_property("表示上限", "-1"));
    assert_eq!(render.limit, 1234);
    assert!(render.set_property("合成", "1"));
    assert_eq!(render.blend, BlendMode::Normal);
    assert!(render.set_property("blend", "0"));
    assert_eq!(render.blend, BlendMode::Additive);
}

#[test]
fn settings_json_round_trips_with_quality_and_render_values() {
    let mut settings = Settings {
        auto_apply: false,
        ..Default::default()
    };
    settings.convert.quality = QualityPreset::Quality;
    settings.convert.fit_mode = FitMode::Original;
    settings.convert.max_points = 0;
    settings.render.limit = 0;
    settings.render.blend = BlendMode::Additive;
    settings.render.fov = 100.0;

    let json = serde_json::to_string_pretty(&settings).unwrap();
    let loaded: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, settings);
    assert_eq!(loaded.convert.quality, QualityPreset::Quality);
    assert_eq!(loaded.render.blend, BlendMode::Additive);
}

#[test]
fn settings_save_and_load_use_the_given_directory() {
    let dir = temp_dir("settings");
    let path = dir.join("settings.json");
    let mut settings = Settings::default();
    settings.convert.max_points = 777;
    settings.render.limit = 0;
    settings.save_to(&path).unwrap();

    let loaded = Settings::load_from(&path);
    assert_eq!(loaded.convert.max_points, 777);
    assert_eq!(loaded.render.limit, 0);
    assert_eq!(loaded.schema_version, 1);

    fs::write(&path, "{ broken").unwrap();
    let fallback = Settings::load_from(&path);
    assert_eq!(fallback, Settings::default());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn overwrite_confirmation_covers_su3ds_and_json() {
    let dir = temp_dir("overwrite");
    let output = dir.join("scene.su3ds");
    assert!(existing_outputs(&output).is_empty());
    fs::write(&output, b"x").unwrap();
    assert_eq!(existing_outputs(&output), vec![output.clone()]);
    fs::write(dir.join("scene.json"), b"{}").unwrap();
    assert_eq!(
        existing_outputs(&output),
        vec![output, dir.join("scene.json")]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn output_paths_are_normalized_to_su3ds() {
    assert_eq!(
        normalized_su3ds_path(PathBuf::from("C:\\work\\scene.ply")),
        PathBuf::from("C:\\work\\scene.su3ds")
    );
    assert_eq!(
        normalized_su3ds_path(PathBuf::from("C:\\work\\scene")),
        PathBuf::from("C:\\work\\scene.su3ds")
    );
    assert_eq!(
        normalized_su3ds_path(PathBuf::from("C:\\work\\scene.SU3DS")),
        PathBuf::from("C:\\work\\scene.SU3DS")
    );
    assert_eq!(format_number(1.0), "1");
    assert_eq!(format_number(0.5), "0.5");
}

#[test]
fn headless_draw_smoke_does_not_need_aviutl() {
    let context = egui::Context::default();
    let mut app = ConverterApp::headless();
    app.settings_mut().convert.quality = QualityPreset::Quality;
    app.settings_mut().render.limit = 0;
    app.settings_mut().render.blend = BlendMode::Additive;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        app.draw(ui);
    });
    assert_eq!(app.settings().convert.quality, QualityPreset::Quality);
    assert_eq!(app.settings().render.limit, 0);
    assert!(!app.is_busy());
}

fn rendered_text(output: &egui::FullOutput, target: &str) -> (String, Option<egui::Pos2>) {
    fn collect(
        shape: &egui::Shape,
        all: &mut String,
        target: &str,
        target_pos: &mut Option<egui::Pos2>,
    ) {
        match shape {
            egui::Shape::Text(text) => {
                let value = text.galley.job.text.as_str();
                all.push_str(value);
                all.push('\n');
                if value == target {
                    *target_pos = Some(text.pos);
                }
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, all, target, target_pos);
                }
            }
            _ => {}
        }
    }

    let mut all = String::new();
    let mut target_pos = None;
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut all, target, &mut target_pos);
    }
    (all, target_pos)
}

fn render(context: &egui::Context, app: &mut ConverterApp) -> egui::FullOutput {
    context.run_ui(egui::RawInput::default(), |ui| app.draw(ui))
}

fn click(context: &egui::Context, app: &mut ConverterApp, pos: egui::Pos2) {
    for pressed in [true, false] {
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::PointerMoved(pos));
        input.events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        });
        let _ = context.run_ui(input, |ui| app.draw(ui));
    }
    let _ = render(context, app);
}

#[test]
fn conversion_details_are_collapsed_in_actual_headless_render() {
    let context = egui::Context::default();
    let mut app = ConverterApp::headless();
    app.settings_mut().convert.quality = QualityPreset::Quality;
    let output = render(&context, &mut app);
    let (text, _) = rendered_text(&output, "変換の詳細設定");

    assert!(
        text.contains("画質"),
        "画質プリセットが常時表示されていません"
    );
    assert!(text.contains("変換の詳細設定"));
    assert!(text.contains("品質優先は処理が遅くなります"));
    assert!(text.contains("表示設定（選択中の3DGS_Su）"));

    for hidden in [
        "点数上限",
        "最大辺",
        "中心を原点に合わせる",
        "Y反転",
        "ディザ",
        "SH（視点依存の色）を保持",
        "選択中へ適用",
    ] {
        assert!(
            !text.contains(hidden),
            "折りたたみ内の {hidden} が初期表示されています"
        );
    }
}

#[test]
fn opening_conversion_details_shows_controls_in_headless_render() {
    let context = egui::Context::default();
    let mut app = ConverterApp::headless();
    let output = render(&context, &mut app);
    let (_, header_pos) = rendered_text(&output, "変換の詳細設定");
    let header_pos = header_pos.expect("変換の詳細設定ヘッダーが描画されていません");
    click(&context, &mut app, header_pos + egui::vec2(2.0, 2.0));

    let output = render(&context, &mut app);
    let (text, _) = rendered_text(&output, "変換の詳細設定");
    for visible in [
        "変換形式（手動）",
        "点数上限",
        "最大辺",
        "全点を使う（点数上限なし）",
    ] {
        assert!(
            text.contains(visible),
            "詳細を開いても {visible} が描画されていません"
        );
    }

    let (_, checkbox_pos) = rendered_text(&output, "全点を使う（点数上限なし）");
    let checkbox_pos = checkbox_pos.expect("全点チェックボックスが描画されていません");
    click(&context, &mut app, checkbox_pos + egui::vec2(2.0, 2.0));
    assert!(
        app.settings().convert.all_points,
        "開いた詳細設定のチェックボックスを操作できません"
    );
}

#[test]
fn opening_render_settings_shows_apply_controls() {
    let context = egui::Context::default();
    let mut app = ConverterApp::headless();
    let output = render(&context, &mut app);
    let (_, pos) = rendered_text(&output, "表示設定（選択中の3DGS_Su）");
    click(
        &context,
        &mut app,
        pos.expect("表示設定ヘッダーがありません") + egui::vec2(2.0, 2.0),
    );
    let output = render(&context, &mut app);
    let (text, _) = rendered_text(&output, "表示設定（選択中の3DGS_Su）");
    for label in [
        "選択中から読込",
        "選択中へ適用",
        "手前クリップ",
        "表示上限（0=全点）",
    ] {
        assert!(
            text.contains(label),
            "表示設定を開いても {label} がありません"
        );
    }
}

#[test]
fn quality_templates_apply_contract_without_touching_unrelated_settings() {
    let mut base = Settings {
        auto_apply: false,
        ..Default::default()
    };
    base.convert.fit_mode = FitMode::Original;
    base.convert.fit_size = 123.5;
    base.convert.scale = 2.5;
    base.convert.center = false;
    base.convert.flip_y = true;
    base.convert.flip_z = true;
    base.render.size = 1.7;
    base.render.alpha = 0.4;
    base.render.near = 4.5;
    base.render.sort = false;
    base.render.ellipse = false;
    base.render.panorama = true;
    base.render.fov = 88.0;
    base.render.blend = BlendMode::Additive;

    for template in QualityTemplate::ALL {
        let mut settings = base.clone();
        settings.apply_quality_template(template);
        assert_eq!(
            settings.matched_quality_template(),
            Some(template),
            "{}",
            template.id()
        );
        let expected = match template {
            QualityTemplate::Preview => (QualityPreset::Lightweight, 30_000, false, true, false),
            QualityTemplate::Balanced => (QualityPreset::Lightweight, 150_000, false, true, false),
            QualityTemplate::Preserve => (QualityPreset::Quality, 0, true, false, true),
        };
        assert_eq!(settings.convert.quality, expected.0);
        assert_eq!(settings.convert.max_points, expected.1);
        assert_eq!(settings.convert.all_points, expected.2);
        assert_eq!(settings.convert.dither, expected.3);
        assert_eq!(settings.convert.keep_sh, expected.4);
        assert_eq!(settings.render.limit, 0);

        assert!(!settings.auto_apply);
        assert_eq!(settings.convert.fit_mode, FitMode::Original);
        assert_eq!(settings.convert.fit_size, 123.5);
        assert_eq!(settings.convert.scale, 2.5);
        assert!(!settings.convert.center);
        assert!(settings.convert.flip_y);
        assert!(settings.convert.flip_z);
        assert_eq!(settings.render.size, 1.7);
        assert_eq!(settings.render.alpha, 0.4);
        assert_eq!(settings.render.near, 4.5);
        assert!(!settings.render.sort);
        assert!(!settings.render.ellipse);
        assert!(settings.render.panorama);
        assert_eq!(settings.render.fov, 88.0);
        assert_eq!(settings.render.blend, BlendMode::Additive);

        let options = settings.convert.to_convert_options();
        assert_eq!(
            options.preserve_quality,
            template == QualityTemplate::Preserve
        );
        assert_eq!(options.keep_sh, expected.4);
    }
}

#[test]
fn quality_template_matching_handles_manual_changes_and_persistence() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.matched_quality_template(),
        Some(QualityTemplate::Preview),
        "従来の既定値はプレビューテンプレートとして表示する"
    );

    settings.apply_quality_template(QualityTemplate::Preview);
    assert_eq!(
        settings.matched_quality_template(),
        Some(QualityTemplate::Preview)
    );

    settings.convert.fit_mode = FitMode::Original;
    settings.convert.scale = 3.0;
    settings.convert.center = false;
    settings.convert.flip_y = true;
    settings.convert.flip_z = true;
    assert_eq!(
        settings.matched_quality_template(),
        Some(QualityTemplate::Preview),
        "テンプレート対象外の変更でカスタムになってはいけない"
    );

    settings.render.limit = 29_999;
    assert_eq!(settings.matched_quality_template(), None);
    settings.render.limit = 30_000;
    assert_eq!(
        settings.matched_quality_template(),
        Some(QualityTemplate::Preview),
        "表示上限が変換上限以上なら一致する"
    );
    settings.render.limit = 0;
    settings.convert.keep_sh = true;
    assert_eq!(
        settings.matched_quality_template(),
        Some(QualityTemplate::Preview),
        "軽量テンプレートの一致判定ではSH設定を無視する"
    );
    settings.convert.keep_sh = false;
    settings.convert.dither = false;
    assert_eq!(settings.matched_quality_template(), None);

    settings.apply_quality_template(QualityTemplate::Preserve);
    let dir = temp_dir("quality_template");
    let path = dir.join("settings.json");
    settings.save_to(&path).unwrap();
    let loaded = Settings::load_from(&path);
    assert_eq!(
        loaded.matched_quality_template(),
        Some(QualityTemplate::Preserve)
    );
    assert_eq!(loaded.render.limit, 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn quality_template_descriptions_state_omitted_information() {
    for template in [QualityTemplate::Preview, QualityTemplate::Balanced] {
        let description = template.description();
        assert!(description.contains("512色"), "{}", template.id());
        assert!(description.contains("8段階"), "{}", template.id());
        assert!(description.contains("SH"), "{}", template.id());
        assert!(description.contains("省略"), "{}", template.id());
    }
    let preserve = QualityTemplate::Preserve.description();
    assert!(preserve.contains("全有効点"));
    assert!(preserve.contains("浮動小数点"));
    assert!(preserve.contains("SH"));
    assert!(preserve.contains("一致"));
}

#[test]
fn quality_template_combo_applies_and_shows_custom_in_headless_render() {
    let context = egui::Context::default();
    let mut app = ConverterApp::headless();
    app.settings_mut().convert.dither = false;
    let output = render(&context, &mut app);
    let (text, combo_pos) = rendered_text(&output, "カスタム");
    assert!(
        text.contains("カスタム"),
        "不一致時のカスタム表記がありません"
    );
    assert!(!text.contains("軽量"));
    assert!(!text.contains("品質優先（追加間引き・量子化なし）"));

    click(
        &context,
        &mut app,
        combo_pos.expect("品質テンプレートComboBoxがありません") + egui::vec2(2.0, 2.0),
    );
    let output = render(&context, &mut app);
    let (text, balanced_pos) = rendered_text(&output, "バランス");
    for label in ["プレビュー", "バランス", "品質保持"] {
        assert!(text.contains(label), "候補 {label} がありません");
    }

    click(
        &context,
        &mut app,
        balanced_pos.expect("バランス候補がありません") + egui::vec2(2.0, 2.0),
    );
    let output = render(&context, &mut app);
    let (text, _) = rendered_text(&output, "バランス");
    assert_eq!(
        app.settings().matched_quality_template(),
        Some(QualityTemplate::Balanced)
    );
    assert_eq!(app.settings().render.limit, 0);
    assert!(!app.settings().convert.keep_sh);
    assert!(text.contains("150,000点"));
    assert!(text.contains("512色"));
    assert!(text.contains("8段階"));
    assert!(text.contains("SH"));

    app.settings_mut().convert.dither = false;
    let output = render(&context, &mut app);
    let (text, _) = rendered_text(&output, "カスタム");
    assert!(
        text.contains("カスタム"),
        "手動変更後にカスタム表記へ戻りません"
    );
}

#[test]
fn reselecting_current_quality_template_applies_canonical_settings() {
    for template in QualityTemplate::ALL {
        let context = egui::Context::default();
        let mut app = ConverterApp::headless();
        app.settings_mut().apply_quality_template(template);
        if template == QualityTemplate::Preserve {
            app.settings_mut().convert.dither = true;
            app.settings_mut().convert.all_points = false;
            app.settings_mut().convert.max_points = 17;
        } else {
            app.settings_mut().render.limit = app.settings().convert.max_points;
        }
        assert_eq!(app.settings().matched_quality_template(), Some(template));
        let mut expected = app.settings().clone();
        expected.apply_quality_template(template);
        let output = render(&context, &mut app);
        let (_, combo_pos) = rendered_text(&output, template.label());
        click(
            &context,
            &mut app,
            combo_pos.unwrap() + egui::vec2(2.0, 2.0),
        );
        let output = render(&context, &mut app);
        let (_, option_pos) = rendered_text(&output, template.label());
        click(
            &context,
            &mut app,
            option_pos.unwrap() + egui::vec2(2.0, 2.0),
        );
        assert_eq!(app.settings().render.limit, 0, "{}", template.id());
        assert_eq!(
            serde_json::to_value(&app.settings().convert).unwrap(),
            serde_json::to_value(&expected.convert).unwrap(),
            "{}",
            template.id()
        );
    }
}

#[test]
fn cli_lists_quality_templates_as_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_su3ds"))
        .arg("--list-quality-templates")
        .output()
        .expect("su3ds を起動できません");
    assert!(
        output.status.success(),
        "exit={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let entries: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(entries.len(), QualityTemplate::ALL.len());

    for (entry, template) in entries.iter().zip(QualityTemplate::ALL) {
        assert_eq!(entry["id"].as_str(), Some(template.id()));
        assert_eq!(entry["label"].as_str(), Some(template.label()));
        assert_eq!(entry["description"].as_str(), Some(template.description()));
        assert_eq!(entry["render_limit"].as_u64(), Some(0));
        let convert = &entry["convert"];
        let expected = match template {
            QualityTemplate::Preview => ("lightweight", 30_000, false, true, false),
            QualityTemplate::Balanced => ("lightweight", 150_000, false, true, false),
            QualityTemplate::Preserve => ("quality", 0, true, false, true),
        };
        assert_eq!(convert["quality"].as_str(), Some(expected.0));
        assert_eq!(convert["max_points"].as_u64(), Some(expected.1));
        assert_eq!(convert["all_points"].as_bool(), Some(expected.2));
        assert_eq!(convert["dither"].as_bool(), Some(expected.3));
        assert_eq!(convert["keep_sh"].as_bool(), Some(expected.4));
    }
}

#[test]
fn old_settings_json_still_loads_with_defaults() {
    let old = r#"{
        "schema_version": 1,
        "auto_apply": false,
        "convert": {"quality": "lightweight", "max_points": 12345},
        "render": {"limit": 0}
    }"#;
    let settings: Settings = serde_json::from_str(old).unwrap();
    assert!(!settings.auto_apply);
    assert_eq!(settings.convert.max_points, 12_345);
    assert_eq!(settings.convert.keep_sh, ConvertSettings::default().keep_sh);
    assert_eq!(settings.render.size, RenderSettings::default().size);
}
