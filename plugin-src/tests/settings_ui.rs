//! 設定モデルとUIの境界テスト。
//!
//! 公開APIだけを使う統合テスト。GUIのホストAPIはヘッドレス構築では呼ばれないため、
//! AviUtl2を起動せずに検証できる。

use std::fs;
use std::path::PathBuf;

use aviutl2_eframe::egui;
use three_dgs_converter_su::gui::{ConverterApp, existing_outputs, normalized_su3ds_path};
use three_dgs_converter_su::settings::{
    BlendMode, ConvertSettings, FitMode, QualityPreset, RENDER_ALPHA, RENDER_BLEND, RENDER_LIMIT,
    RENDER_SIZE, RenderSettings, Settings, format_number,
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
