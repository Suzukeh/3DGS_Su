//! 3DGS変換UIの永続設定と、AviUtl2オブジェクトへ読み書きする表示設定。
//!
//! 設定ファイルは `%LOCALAPPDATA%\3DGSConverter_Su\settings.json`。
//! 検証時は環境変数 `THREEDGS_SETTINGS_DIR` で保存先ディレクトリを差し替えられる。
//! AviUtl2 の `app_data_path()` には依存しない。

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::ConvertOptions;

pub const SETTINGS_DIR_ENV: &str = "THREEDGS_SETTINGS_DIR";
pub const SETTINGS_DIR_NAME: &str = "3DGSConverter_Su";
pub const SETTINGS_FILE_NAME: &str = "settings.json";

/// 3DGS_Su の「3DGSファイル(.su3ds)」項目名。
pub const FILE_ITEM: &str = "3DGSファイル(.su3ds)";

pub const RENDER_SIZE: &str = "サイズ";
pub const RENDER_ALPHA: &str = "不透明度";
pub const RENDER_NEAR: &str = "手前クリップ";
pub const RENDER_LIMIT: &str = "表示上限";
pub const RENDER_SORT: &str = "奥から描く";
pub const RENDER_ELLIPSE: &str = "楕円";
pub const RENDER_PANORAMA: &str = "パノラマ";
pub const RENDER_FOV: &str = "画角";
pub const RENDER_BLEND: &str = "合成";

/// 画質プリセット。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityPreset {
    /// 従来どおり点数を絞り、色と不透明度を量子化する。
    #[default]
    Lightweight,
    /// 全点・浮動小数点RGB/alpha・SHを保持する。処理は遅い。
    Quality,
}

impl QualityPreset {
    pub const ALL: [Self; 2] = [Self::Lightweight, Self::Quality];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lightweight => "軽量",
            Self::Quality => "品質優先（追加間引き・量子化なし）",
        }
    }

    pub fn is_quality(self) -> bool {
        self == Self::Quality
    }
}

/// UIとCLIで共有する変換テンプレート。識別子と設定値を一元管理する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityTemplate {
    Preview,
    Balanced,
    Preserve,
}

impl QualityTemplate {
    pub const ALL: [Self; 3] = [Self::Preview, Self::Balanced, Self::Preserve];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Balanced => "balanced",
            Self::Preserve => "preserve",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Preview => "プレビュー",
            Self::Balanced => "バランス",
            Self::Preserve => "品質保持",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Preview => "最大30,000点。RGBは512色、不透明度は8段階に量子化。SHは省略します。",
            Self::Balanced => {
                "最大150,000点。RGBは512色、不透明度は8段階に量子化。SHは省略します。"
            }
            Self::Preserve => {
                "全有効点を保持。浮動小数点RGB/alphaとSHを保持します。処理が遅く、元画像と見た目が一致する保証はありません。"
            }
        }
    }
}

/// 位置の正規化方法。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    /// 最大辺が `fit_size` になるよう正規化する（fit > 0）。
    #[default]
    Normalized,
    /// 元ファイルのスケールを保つ（fit = 0）。
    Original,
}

impl FitMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normalized => "最大辺を正規化",
            Self::Original => "元のサイズを保つ (fit=0)",
        }
    }
}

/// 3DGS_Su の合成モード。AviUtl2 へは日本語名で読み書きする。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Additive,
}

impl BlendMode {
    pub const ALL: [Self; 2] = [Self::Normal, Self::Additive];

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "通常",
            Self::Additive => "加算",
        }
    }
}

/// 変換オプション。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConvertSettings {
    pub quality: QualityPreset,
    /// 点数上限なし。品質優先でも常に全点。
    pub all_points: bool,
    /// 軽量時の点数上限。0 も無制限（UIでは全点スイッチを推奨）。
    pub max_points: usize,
    pub fit_mode: FitMode,
    pub fit_size: f64,
    pub scale: f64,
    /// 中心を原点へ移動する（ConvertOptions.no_center の反転）。
    pub center: bool,
    pub flip_y: bool,
    pub flip_z: bool,
    /// 色のディザ（ConvertOptions.no_dither の反転）。
    pub dither: bool,
    pub keep_sh: bool,
}

impl Default for ConvertSettings {
    fn default() -> Self {
        Self {
            quality: QualityPreset::Lightweight,
            all_points: false,
            max_points: 30_000,
            fit_mode: FitMode::Normalized,
            fit_size: 400.0,
            scale: 1.0,
            center: true,
            flip_y: false,
            flip_z: false,
            dither: true,
            keep_sh: true,
        }
    }
}

impl ConvertSettings {
    /// 品質優先・全点指定なら 0（無制限）。それ以外はユーザー指定値。
    pub fn effective_max(&self) -> usize {
        if self.quality.is_quality() || self.all_points {
            0
        } else {
            self.max_points
        }
    }

    pub fn uses_all_points(&self) -> bool {
        self.effective_max() == 0
    }

    pub fn effective_fit(&self) -> f64 {
        match self.fit_mode {
            FitMode::Normalized => self.fit_size,
            FitMode::Original => 0.0,
        }
    }

    /// バックエンドの `ConvertOptions` へ変換する。
    ///
    /// 品質優先では点数上限とディザ設定を無視し、全点・ディザなしにする。SHは明示指定に従う。
    pub fn to_convert_options(&self) -> ConvertOptions {
        let quality = self.quality.is_quality();
        ConvertOptions {
            max: self.effective_max(),
            scale: self.scale,
            fit: self.effective_fit(),
            flip_y: self.flip_y,
            flip_z: self.flip_z,
            no_center: !self.center,
            no_dither: quality || !self.dither,
            preserve_quality: quality,
            keep_sh: self.keep_sh,
        }
    }
}

/// 選択オブジェクトの表示設定。変更は明示的な適用ボタンを押したときだけ反映する。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderSettings {
    pub size: f32,
    pub alpha: f32,
    pub near: f32,
    /// 0 は全点。
    pub limit: usize,
    pub sort: bool,
    pub ellipse: bool,
    pub panorama: bool,
    pub fov: f32,
    pub blend: BlendMode,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            size: 1.0,
            alpha: 1.0,
            near: 10.0,
            limit: 30_000,
            sort: true,
            ellipse: true,
            panorama: false,
            fov: 75.0,
            blend: BlendMode::Normal,
        }
    }
}

impl RenderSettings {
    /// オブジェクトへ設定する全項目。名前は3DGS_Suの表示名に合わせる。
    pub fn to_properties(&self) -> Vec<(String, String)> {
        vec![
            (RENDER_SIZE.into(), format_number(self.size as f64)),
            (RENDER_ALPHA.into(), format_number(self.alpha as f64)),
            (RENDER_NEAR.into(), format_number(self.near as f64)),
            (RENDER_LIMIT.into(), self.limit.to_string()),
            (RENDER_SORT.into(), bool_value(self.sort).into()),
            (RENDER_ELLIPSE.into(), bool_value(self.ellipse).into()),
            (RENDER_PANORAMA.into(), bool_value(self.panorama).into()),
            (RENDER_FOV.into(), format_number(self.fov as f64)),
            (RENDER_BLEND.into(), self.blend.label().into()),
        ]
    }

    /// `read_properties()` の戻り値を取り込む。取り込めた項目数を返す。
    pub fn merge_properties<I, S1, S2>(&mut self, properties: I) -> usize
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        properties
            .into_iter()
            .filter(|(name, value)| self.set_property(name.as_ref(), value.as_ref()))
            .count()
    }

    /// 1項目だけ取り込む。未知の名前や解析不能な値なら false。
    pub fn set_property(&mut self, name: &str, value: &str) -> bool {
        let Some(key) = canonical_render_key(name) else {
            return false;
        };
        match key {
            RENDER_SIZE => set_f32(&mut self.size, value, 0.01, 10.0),
            RENDER_ALPHA => set_f32(&mut self.alpha, value, 0.0, 1.0),
            RENDER_NEAR => set_f32(&mut self.near, value, 0.1, 100_000.0),
            RENDER_LIMIT => set_limit(&mut self.limit, value),
            RENDER_SORT => set_bool(&mut self.sort, value),
            RENDER_ELLIPSE => set_bool(&mut self.ellipse, value),
            RENDER_PANORAMA => set_bool(&mut self.panorama, value),
            RENDER_FOV => set_f32(&mut self.fov, value, 10.0, 170.0),
            RENDER_BLEND => {
                let Some(blend) = parse_blend(value) else {
                    return false;
                };
                self.blend = blend;
                true
            }
            _ => false,
        }
    }

    /// 表示設定だけを保存するときの互換用。
    pub fn normalized(mut self) -> Self {
        self.size = finite_or(self.size, 1.0).clamp(0.01, 10.0);
        self.alpha = finite_or(self.alpha, 1.0).clamp(0.0, 1.0);
        self.near = finite_or(self.near, 10.0).clamp(0.1, 100_000.0);
        self.fov = finite_or(self.fov, 75.0).clamp(10.0, 170.0);
        self
    }
}

/// 永続設定のルート。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub auto_apply: bool,
    pub convert: ConvertSettings,
    pub render: RenderSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            auto_apply: true,
            convert: ConvertSettings::default(),
            render: RenderSettings::default(),
        }
    }
}

impl Settings {
    /// テンプレートが変換・表示設定に反映される項目だけを更新する。
    pub fn apply_quality_template(&mut self, template: QualityTemplate) {
        self.convert.quality = match template {
            QualityTemplate::Preview | QualityTemplate::Balanced => QualityPreset::Lightweight,
            QualityTemplate::Preserve => QualityPreset::Quality,
        };
        match template {
            QualityTemplate::Preview => {
                self.convert.max_points = 30_000;
                self.convert.all_points = false;
                self.convert.dither = true;
                self.convert.keep_sh = false;
            }
            QualityTemplate::Balanced => {
                self.convert.max_points = 150_000;
                self.convert.all_points = false;
                self.convert.dither = true;
                self.convert.keep_sh = false;
            }
            QualityTemplate::Preserve => {
                self.convert.max_points = 0;
                self.convert.all_points = true;
                self.convert.dither = false;
                self.convert.keep_sh = true;
            }
        }
        self.render.limit = 0;
    }

    /// 現在の有効設定に一致するテンプレート。個別の配置や表示項目は判定しない。
    pub fn matched_quality_template(&self) -> Option<QualityTemplate> {
        let convert = &self.convert;
        if convert.quality.is_quality() {
            return (convert.keep_sh && self.render.limit == 0)
                .then_some(QualityTemplate::Preserve);
        }
        if !convert.dither {
            return None;
        }
        let effective_max = convert.effective_max();
        [QualityTemplate::Preview, QualityTemplate::Balanced]
            .into_iter()
            .find(|template| {
                let max = match template {
                    QualityTemplate::Preview => 30_000,
                    QualityTemplate::Balanced => 150_000,
                    QualityTemplate::Preserve => unreachable!(),
                };
                effective_max == max && (self.render.limit == 0 || self.render.limit >= max)
            })
    }

    /// 保存先ディレクトリ。`THREEDGS_SETTINGS_DIR` があればそれを優先する。
    pub fn settings_dir() -> PathBuf {
        if let Some(path) = env::var_os(SETTINGS_DIR_ENV) {
            let path = PathBuf::from(path);
            if !path.as_os_str().is_empty() {
                return path;
            }
        }
        let base = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(env::temp_dir);
        base.join(SETTINGS_DIR_NAME)
    }

    pub fn settings_path() -> PathBuf {
        Self::settings_dir().join(SETTINGS_FILE_NAME)
    }

    pub fn load() -> Self {
        Self::load_from(&Self::settings_path())
    }

    /// 壊れたJSONや存在しないファイルでは既定値に戻す。
    pub fn load_from(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .unwrap_or_default()
            .normalized()
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::settings_path())
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json =
            serde_json::to_string_pretty(&self.clone().normalized()).map_err(io::Error::other)?;
        fs::write(path, json)
    }

    pub fn normalized(mut self) -> Self {
        self.schema_version = 1;
        self.convert.scale = finite_or_f64(self.convert.scale, 1.0).clamp(0.001, 1_000.0);
        self.convert.fit_size = finite_or_f64(self.convert.fit_size, 400.0).clamp(1.0, 1_000_000.0);
        self.render = self.render.normalized();
        self
    }
}

fn canonical_render_key(name: &str) -> Option<&'static str> {
    match name.trim() {
        RENDER_SIZE | "size" => Some(RENDER_SIZE),
        RENDER_ALPHA | "alpha" => Some(RENDER_ALPHA),
        RENDER_NEAR | "near" => Some(RENDER_NEAR),
        RENDER_LIMIT | "limit" => Some(RENDER_LIMIT),
        RENDER_SORT | "sort" => Some(RENDER_SORT),
        RENDER_ELLIPSE | "ellipse" => Some(RENDER_ELLIPSE),
        RENDER_PANORAMA | "panorama" => Some(RENDER_PANORAMA),
        RENDER_FOV | "fov" => Some(RENDER_FOV),
        RENDER_BLEND | "blend" => Some(RENDER_BLEND),
        _ => None,
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn finite_or_f64(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

fn set_f32(target: &mut f32, value: &str, min: f32, max: f32) -> bool {
    let Ok(parsed) = value.trim().parse::<f32>() else {
        return false;
    };
    if !parsed.is_finite() {
        return false;
    }
    *target = parsed.clamp(min, max);
    true
}

fn set_limit(target: &mut usize, value: &str) -> bool {
    let Ok(parsed) = value.trim().parse::<f64>() else {
        return false;
    };
    if !parsed.is_finite() || parsed < 0.0 {
        return false;
    }
    *target = parsed.round().min(usize::MAX as f64) as usize;
    true
}

fn set_bool(target: &mut bool, value: &str) -> bool {
    match parse_bool(value) {
        Some(value) => {
            *target = value;
            true
        }
        None => false,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "1" | "true" | "on" | "yes" | "オン" | "はい" => Some(true),
        "0" | "false" | "off" | "no" | "オフ" | "いいえ" => Some(false),
        _ => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(|value| value != 0.0),
    }
}

fn parse_blend(value: &str) -> Option<BlendMode> {
    let value = value.trim().to_ascii_lowercase();
    if value.contains("加算") || value.contains("add") || value == "0" {
        Some(BlendMode::Additive)
    } else if value.contains("通常") || value.contains("normal") || value == "1" {
        Some(BlendMode::Normal)
    } else {
        None
    }
}

fn bool_value(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

/// 値の表示用。整数は小数点を付けず、小数は末尾0を落とす。
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    if value == 0.0 {
        return "0".into();
    }
    if value.fract().abs() < 1e-9 {
        return format!("{value:.0}");
    }
    let mut text = format!("{value:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}
