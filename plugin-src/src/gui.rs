//! 3DGS変換の専用UI。
//!
//! メニューを辿らず、このウィンドウだけで
//! 「入力選択 → 設定 → 変換 → 選択オブジェクトへ適用」まで完結させる。
//! 変換は純Rustのバックグラウンドスレッドで行い、UIスレッドからAviUtl2 APIを
//! 呼ぶのは対象の取得・適用と状態確認だけに限定する。

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use aviutl2::generic::ObjectHandle;
use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use native_dialog::FileDialogBuilder;

use crate::settings::{BlendMode, FILE_ITEM, FitMode, QualityPreset, RENDER_LIMIT, Settings};
use crate::{
    ConvertInfo, ConvertOptions, apply_properties, capture_targets, convert_file, current_epoch,
    last_file_path, pick_file, read_properties, selected_count, write_last_file,
};

const SETTINGS_HINT: &str = "設定は自動保存されます。";

#[cfg(test)]
#[path = "gui_tests.rs"]
mod tests;
const QUALITY_WARNING: &str = "品質優先は処理が遅くなります。GPUで表示していた元画像と完全に同じ見た目になる保証はありません。";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Idle,
    Busy,
    Complete,
    Error,
}

impl StatusKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Busy => "busy",
            Self::Complete => "complete",
            Self::Error => "error",
        }
    }
}

struct PendingConversion {
    input: PathBuf,
    output: PathBuf,
    options: ConvertOptions,
    targets: Vec<ObjectHandle>,
    epoch: u64,
    quality: bool,
    auto_apply: bool,
    existing: Vec<PathBuf>,
}

struct ConversionJob {
    receiver: Receiver<Result<ConvertInfo, String>>,
    handle: Option<JoinHandle<()>>,
    targets: Vec<ObjectHandle>,
    epoch: u64,
    output: PathBuf,
    quality: bool,
    auto_apply: bool,
}

impl ConversionJob {
    fn start(pending: PendingConversion) -> Result<Self, String> {
        let PendingConversion {
            input,
            output,
            options,
            targets,
            epoch,
            quality,
            auto_apply,
            ..
        } = pending;
        let (sender, receiver) = mpsc::channel();
        let worker_output = output.clone();
        let handle = thread::Builder::new()
            .name("3dgs-convert".into())
            .spawn(move || {
                // ワーカー内ではAviUtl2のホストAPIを一切呼ばない。
                let result = convert_file(&input, &worker_output, &options)
                    .map_err(|error| error.to_string());
                let _ = sender.send(result);
            })
            .map_err(|error| format!("変換スレッドを開始できません: {error}"))?;
        Ok(Self {
            receiver,
            handle: Some(handle),
            targets,
            epoch,
            output,
            quality,
            auto_apply,
        })
    }

    fn poll(&self) -> Option<Result<ConvertInfo, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err("変換ワーカーが結果を返さず終了しました。".into()))
            }
        }
    }

    fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// 変換UI。
pub struct ConverterApp {
    handle: Option<AviUtl2EframeHandle>,
    input: String,
    output: String,
    output_auto: bool,
    settings: Settings,
    saved_settings: Settings,
    settings_path: PathBuf,
    persist_settings: bool,
    status_kind: StatusKind,
    status_text: String,
    job: Option<ConversionJob>,
    pending_overwrite: Option<PendingConversion>,
    started_at: Option<Instant>,
    last_info: Option<ConvertInfo>,
    last_applied: usize,
    #[cfg(feature = "selftest")]
    selftest: SelftestState,
}

impl ConverterApp {
    pub fn new(cc: &eframe::CreationContext<'_>, handle: AviUtl2EframeHandle) -> Self {
        cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());
        let settings = Settings::load();
        let mut app = Self::with_settings(Some(handle), settings, true);
        if let Ok(input) = std::env::var("THREEDGS_TEST_INPUT")
            && !input.trim().is_empty()
        {
            app.set_input_path(input);
            app.sync_output();
        }
        app
    }

    /// AviUtl2のウィンドウを作らず、設定と描画だけを検証するための構築子。
    pub fn headless() -> Self {
        Self::with_settings(None, Settings::default(), false)
    }

    fn with_settings(
        handle: Option<AviUtl2EframeHandle>,
        settings: Settings,
        persist_settings: bool,
    ) -> Self {
        Self {
            handle,
            input: String::new(),
            output: String::new(),
            output_auto: true,
            saved_settings: settings.clone(),
            settings,
            settings_path: Settings::settings_path(),
            persist_settings,
            status_kind: StatusKind::Idle,
            status_text: "3DGS_Su オブジェクトを選択してから使います。".into(),
            job: None,
            pending_overwrite: None,
            started_at: None,
            last_info: None,
            last_applied: 0,
            #[cfg(feature = "selftest")]
            selftest: SelftestState::default(),
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn status(&self) -> (StatusKind, &str) {
        (self.status_kind, &self.status_text)
    }

    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    pub fn last_info(&self) -> Option<&ConvertInfo> {
        self.last_info.as_ref()
    }

    pub fn last_applied(&self) -> usize {
        self.last_applied
    }

    pub fn set_input_path(&mut self, path: impl AsRef<Path>) {
        self.input = path.as_ref().to_string_lossy().into_owned();
    }

    pub fn set_output_path(&mut self, path: impl AsRef<Path>) {
        self.output = path.as_ref().to_string_lossy().into_owned();
        self.output_auto = false;
    }

    pub fn set_paths(&mut self, input: impl AsRef<Path>, output: impl AsRef<Path>) {
        self.set_input_path(input);
        self.set_output_path(output);
    }

    fn set_status(&mut self, kind: StatusKind, text: impl Into<String>) {
        self.status_kind = kind;
        self.status_text = text.into();
    }

    fn sync_output(&mut self) {
        let input = self.input.trim();
        self.output = if input.is_empty() {
            String::new()
        } else {
            PathBuf::from(input)
                .with_extension("su3ds")
                .to_string_lossy()
                .into_owned()
        };
        self.output_auto = true;
    }

    /// ユーザーが出力欄を直接編集していない場合だけ自動更新する。
    fn sync_output_if_auto(&mut self) {
        if self.output_auto || self.output.trim().is_empty() {
            self.sync_output();
        }
    }

    fn choose_input_file(&mut self) {
        match pick_file("3DGSを変換", &["sog", "ply"]) {
            Ok(Some(path)) => {
                self.set_input_path(&path);
                self.sync_output_if_auto();
                self.set_status(StatusKind::Idle, "入力ファイルを選びました。");
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(StatusKind::Error, format!("ファイルを選べません: {error}"));
            }
        }
    }

    fn choose_input_folder(&mut self) {
        let builder = self
            .dialog_builder("SOGフォルダを選択")
            .set_title("SOGフォルダを選択");
        match builder.open_single_dir().show() {
            Ok(Some(path)) => {
                self.set_input_path(&path);
                self.sync_output_if_auto();
                self.set_status(StatusKind::Idle, "SOGフォルダを選びました。");
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(StatusKind::Error, format!("フォルダを選べません: {error}"));
            }
        }
    }

    fn choose_output(&mut self) {
        let current = PathBuf::from(self.output.trim());
        let mut builder = self.dialog_builder("出力先を選択");
        builder = builder
            .set_title("出力先を選択")
            .add_filter("3DGSファイル", ["su3ds"]);
        if let Some(name) = current.file_name() {
            builder = builder.set_filename(name.to_string_lossy());
        }
        if let Some(parent) = current.parent().filter(|parent| parent.is_dir()) {
            builder = builder.set_location(parent);
        }
        match builder.save_single_file().show() {
            Ok(Some(path)) => {
                let path = ensure_su3ds(path);
                self.set_output_path(path);
                self.set_status(StatusKind::Idle, "出力先を選びました。");
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(StatusKind::Error, format!("保存先を選べません: {error}"));
            }
        }
    }

    fn dialog_builder(&self, title: &str) -> FileDialogBuilder {
        let mut builder = FileDialogBuilder::default().set_title(title);
        if let Some(handle) = &self.handle {
            builder = builder.set_owner(handle);
        }
        builder
    }

    fn accepted_input(&self) -> Result<PathBuf, String> {
        let text = self.input.trim();
        if text.is_empty() {
            return Err("入力ファイルまたはSOGフォルダを指定してください。".into());
        }
        let path = PathBuf::from(text);
        if path.is_dir() {
            return Ok(path);
        }
        if !path.is_file() {
            return Err("入力ファイルが見つかりません。".into());
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension != "ply" && extension != "sog" {
            return Err("入力は .ply または .sog / SOGフォルダを選んでください。".into());
        }
        Ok(path)
    }

    fn normalized_output(&mut self) -> Result<PathBuf, String> {
        let text = self.output.trim();
        if text.is_empty() {
            return Err("出力先を指定してください。".into());
        }
        let path = ensure_su3ds(PathBuf::from(text));
        if path.is_dir() {
            return Err("出力先がフォルダです。ファイル名を指定してください。".into());
        }
        self.output = path.to_string_lossy().into_owned();
        self.output_auto = false;
        Ok(path)
    }

    fn request_conversion(&mut self) {
        if self.job.is_some() || self.pending_overwrite.is_some() {
            return;
        }
        let input = match self.accepted_input() {
            Ok(path) => path,
            Err(error) => {
                self.set_status(StatusKind::Error, error);
                return;
            }
        };
        let output = match self.normalized_output() {
            Ok(path) => path,
            Err(error) => {
                self.set_status(StatusKind::Error, error);
                return;
            }
        };
        let targets = match capture_targets() {
            Ok(targets) => targets,
            Err(error) => {
                self.set_status(
                    StatusKind::Error,
                    format!("選択オブジェクトを取得できません: {error}"),
                );
                return;
            }
        };
        let pending = PendingConversion {
            input,
            output: output.clone(),
            options: self.settings.convert.to_convert_options(),
            targets,
            epoch: current_epoch(),
            quality: self.settings.convert.quality.is_quality(),
            auto_apply: self.settings.auto_apply,
            existing: existing_outputs(&output),
        };
        if pending.existing.is_empty() {
            self.begin_conversion(pending);
        } else {
            self.set_status(
                StatusKind::Idle,
                "出力先に既存ファイルがあります。上書き確認をしてください。",
            );
            self.pending_overwrite = Some(pending);
        }
    }

    fn begin_conversion(&mut self, pending: PendingConversion) {
        self.pending_overwrite = None;
        self.output = pending.output.to_string_lossy().into_owned();
        self.output_auto = false;
        self.last_info = None;
        self.last_applied = 0;
        match ConversionJob::start(pending) {
            Ok(job) => {
                self.job = Some(job);
                self.started_at = Some(Instant::now());
                self.set_status(
                    StatusKind::Busy,
                    "変換しています…（進捗率は取得できません）",
                );
            }
            Err(error) => {
                self.started_at = None;
                self.set_status(StatusKind::Error, format!("変換を開始できません: {error}"));
            }
        }
    }

    fn poll_conversion(&mut self) {
        let Some(result) = self.job.as_ref().and_then(ConversionJob::poll) else {
            return;
        };
        let Some(mut job) = self.job.take() else {
            return;
        };
        job.join();
        self.finish_conversion(job, result);
    }

    fn finish_conversion(&mut self, job: ConversionJob, result: Result<ConvertInfo, String>) {
        self.started_at = None;
        let ConversionJob {
            targets,
            epoch,
            output,
            quality,
            auto_apply,
            ..
        } = job;
        let info = match result {
            Ok(info) => info,
            Err(error) => {
                self.set_status(StatusKind::Error, format!("変換に失敗しました: {error}"));
                return;
            }
        };
        let points_in_source = info.points_in_source;
        let points_written = info.points_written;
        let elapsed_ms = info.elapsed_ms;
        self.last_info = Some(info);

        let mut note = String::new();
        if self.handle.is_some()
            && let Err(error) = write_last_file(&output)
        {
            note.push_str(&format!(" / 直前ファイルの記録に失敗: {error}"));
        }

        let mut applied = 0usize;
        let mut status = StatusKind::Complete;
        if auto_apply {
            if current_epoch() != epoch {
                note.push_str(" / シーンが変わったため自動適用しませんでした");
            } else if targets.is_empty() {
                status = StatusKind::Error;
                note.push_str(" / 選択中の 3DGS_Su がありません");
            } else {
                let mut properties =
                    vec![(FILE_ITEM.to_string(), output.to_string_lossy().into_owned())];
                if quality {
                    properties.push((RENDER_LIMIT.to_string(), "0".to_string()));
                }
                match apply_properties(targets, properties) {
                    Ok(count) if count > 0 => {
                        applied = count;
                        if quality {
                            self.settings.render.limit = 0;
                            note.push_str(&format!(" / {count} 件に適用（高品質: 表示上限=0）"));
                        } else {
                            note.push_str(&format!(" / {count} 件に適用"));
                        }
                    }
                    Ok(_) => {
                        status = StatusKind::Error;
                        note.push_str(" / 選択中の 3DGS_Su に適用できませんでした");
                    }
                    Err(error) => {
                        status = StatusKind::Error;
                        note.push_str(&format!(" / 適用に失敗: {error}"));
                    }
                }
            }
        }
        self.last_applied = applied;
        self.set_status(
            status,
            format!(
                "完了: 入力 {points_in_source} 点 → 出力 {points_written} 点 / {elapsed_ms} ms{note}"
            ),
        );
    }

    fn apply_path(&mut self, path: &Path) {
        if self.job.is_some() {
            self.set_status(StatusKind::Busy, "変換の完了後に適用してください。");
            return;
        }
        let targets = match capture_targets() {
            Ok(targets) => targets,
            Err(error) => {
                self.set_status(
                    StatusKind::Error,
                    format!("選択オブジェクトを取得できません: {error}"),
                );
                return;
            }
        };
        if targets.is_empty() {
            self.set_status(
                StatusKind::Error,
                "3DGS_Su オブジェクトが選択されていません。",
            );
            return;
        }
        let value = path.to_string_lossy().into_owned();
        match apply_properties(targets, vec![(FILE_ITEM.to_string(), value)]) {
            Ok(0) => {
                self.set_status(
                    StatusKind::Error,
                    "3DGS_Su オブジェクトに適用できませんでした。",
                );
            }
            Ok(count) => {
                self.output = path.to_string_lossy().into_owned();
                self.output_auto = false;
                self.last_applied = count;
                if let Err(error) = write_last_file(path) {
                    self.set_status(
                        StatusKind::Complete,
                        format!("{count} 件に適用しました（直前ファイルの記録に失敗: {error}）"),
                    );
                } else {
                    self.set_status(StatusKind::Complete, format!("{count} 件に適用しました。"));
                }
            }
            Err(error) => {
                self.set_status(StatusKind::Error, format!("適用に失敗しました: {error}"));
            }
        }
    }

    fn apply_existing(&mut self) {
        match pick_file("変換済みの .su3ds を選択", &["su3ds"]) {
            Ok(Some(path)) => self.apply_path(&path),
            Ok(None) => {}
            Err(error) => {
                self.set_status(StatusKind::Error, format!("ファイルを選べません: {error}"));
            }
        }
    }

    fn apply_last(&mut self) {
        match last_file_path().filter(|path| path.is_file()) {
            Some(path) => self.apply_path(&path),
            None => self.set_status(StatusKind::Error, "直前の変換結果がありません。"),
        }
    }

    fn read_render_settings(&mut self) {
        match read_properties() {
            Ok(properties) => {
                let count = self.settings.render.merge_properties(properties);
                if count == 0 {
                    self.set_status(
                        StatusKind::Error,
                        "3DGS_Su の表示設定を読み込めませんでした。",
                    );
                } else {
                    self.set_status(
                        StatusKind::Complete,
                        format!("表示設定を {count} 項目読み込みました。"),
                    );
                }
            }
            Err(error) => {
                self.set_status(
                    StatusKind::Error,
                    format!("表示設定の読み込みに失敗しました: {error}"),
                );
            }
        }
    }

    fn apply_render_settings(&mut self) {
        let targets = match capture_targets() {
            Ok(targets) => targets,
            Err(error) => {
                self.set_status(
                    StatusKind::Error,
                    format!("選択オブジェクトを取得できません: {error}"),
                );
                return;
            }
        };
        if targets.is_empty() {
            self.set_status(
                StatusKind::Error,
                "3DGS_Su オブジェクトが選択されていません。",
            );
            return;
        }
        match apply_properties(targets, self.settings.render.to_properties()) {
            Ok(0) => {
                self.set_status(StatusKind::Error, "表示設定を適用できませんでした。");
            }
            Ok(count) => {
                self.last_applied = count;
                self.set_status(
                    StatusKind::Complete,
                    format!("{count} 件に表示設定を適用しました。"),
                );
            }
            Err(error) => {
                self.set_status(
                    StatusKind::Error,
                    format!("表示設定の適用に失敗しました: {error}"),
                );
            }
        }
    }

    fn handle_dropped_files(&mut self, ui: &egui::Ui) {
        let dropped = ui.ctx().input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .find_map(|file| file.path.clone())
        });
        let Some(path) = dropped else {
            return;
        };
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if path.is_dir() || extension == "ply" || extension == "sog" {
            self.set_input_path(&path);
            self.sync_output_if_auto();
            self.set_status(StatusKind::Idle, "入力ファイルを選びました。");
        } else if extension == "su3ds" {
            self.apply_path(&path);
        } else {
            self.set_status(StatusKind::Error, "対応していないファイルです。");
        }
    }

    fn persist_if_changed(&mut self) {
        if !self.persist_settings || self.settings == self.saved_settings {
            return;
        }
        match self.settings.save_to(&self.settings_path) {
            Ok(()) => {
                self.saved_settings = self.settings.clone();
            }
            Err(error) => {
                // 保存失敗を毎フレーム繰り返さないよう、次に変更されるまで待つ。
                self.saved_settings = self.settings.clone();
                self.set_status(StatusKind::Error, format!("設定を保存できません: {error}"));
            }
        }
    }

    /// ヘッドレス試験を含む描画処理。
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.poll_conversion();
        #[cfg(feature = "selftest")]
        self.selftest_tick();
        self.handle_dropped_files(ui);
        let selected = selected_count().unwrap_or(0);
        let busy = self.job.is_some();
        if busy {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("3DGS変換");
                ui.label("3DGSの .ply / .sog を .su3ds に変換し、選択中の 3DGS_Su に設定します。");

                egui::CollapsingHeader::new("使い方と注意（クリックで開く）")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.small("1. 入力ファイルまたはSOGフォルダを選びます。");
                        ui.small("2. 画質と変換設定を選び、出力先を確認します。");
                        ui.small("3. 「変換して適用」を押します。");
                        ui.small("4. 表示設定は「選択中へ適用」を押したときだけ反映されます。");
                        ui.small("既存の .su3ds / .json がある場合は上書き確認が出ます。");
                        ui.small("変換中は画面を閉じるまで完了を待ってください（中止機能はありません）。");
                    });

                ui.add_space(4.0);
                if selected == 0 {
                    ui.colored_label(
                        egui::Color32::from_rgb(230, 170, 90),
                        "3DGS_Su オブジェクトを選択してください（タイムラインまたはオブジェクトリスト）。",
                    );
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(120, 200, 140),
                        format!("選択中: {selected} 件"),
                    );
                }

                ui.separator();
                self.draw_paths(ui);
                self.draw_conversion_settings(ui);
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(
                        &mut self.settings.auto_apply,
                        "変換後に選択中へ自動適用",
                    );
                    if self.settings.convert.quality.is_quality() {
                        ui.small("高品質の自動適用時は表示上限=0になります。");
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    let primary = if self.settings.auto_apply {
                        "変換して適用"
                    } else {
                        "変換"
                    };
                    if ui
                        .add_enabled(
                            !busy && self.pending_overwrite.is_none(),
                            egui::Button::new(primary)
                                .min_size(egui::vec2(130.0, 30.0))
                                .wrap(),
                        )
                        .clicked()
                    {
                        self.request_conversion();
                    }
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new("既存の .su3ds を適用...").wrap(),
                        )
                        .clicked()
                    {
                        self.apply_existing();
                    }
                    if ui
                        .add_enabled(!busy, egui::Button::new("直前の結果を適用").wrap())
                        .clicked()
                    {
                        self.apply_last();
                    }
                });

                ui.add_space(6.0);
                self.draw_status(ui);
                ui.separator();
                self.draw_render_settings(ui);
                ui.add_space(8.0);
                ui.small(SETTINGS_HINT);
            });

        self.draw_overwrite_modal(ui.ctx());
        self.persist_if_changed();
    }

    fn draw_paths(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("入力");
            let width = (ui.available_width() - 190.0).max(140.0);
            let response = ui.add_sized(
                [width, 22.0],
                egui::TextEdit::singleline(&mut self.input).hint_text(".ply / .sog / SOGフォルダ"),
            );
            if response.changed() && (self.output_auto || self.output.trim().is_empty()) {
                self.sync_output();
            }
            if ui.button("ファイル参照...").clicked() {
                self.choose_input_file();
            }
            if ui.button("SOGフォルダ...").clicked() {
                self.choose_input_folder();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("出力");
            let width = (ui.available_width() - 110.0).max(140.0);
            let response = ui.add_sized(
                [width, 22.0],
                egui::TextEdit::singleline(&mut self.output).hint_text("出力 .su3ds"),
            );
            if response.changed() {
                self.output_auto = false;
            }
            if ui.button("保存先...").clicked() {
                self.choose_output();
            }
        });
        ui.small("同名の .json も出力されます。保存先は出力欄で直接編集できます。");
    }

    fn draw_conversion_settings(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.strong("画質");
            for preset in QualityPreset::ALL {
                if ui.radio_value(&mut self.settings.convert.quality, preset, preset.label()).changed() && preset.is_quality() {
                    self.settings.convert.keep_sh = true;
                }
            }
            if self.settings.convert.quality.is_quality() {
                ui.colored_label(egui::Color32::from_rgb(230, 170, 90), QUALITY_WARNING);
                ui.small("品質優先では点数上限・ディザ設定を無視し、全点を保持します。SHは既定でオンです。");
            }

            ui.add_space(4.0);
            let quality = self.settings.convert.quality.is_quality();
            if quality {
                let mut forced_all = true;
                ui.add_enabled(
                    false,
                    egui::Checkbox::new(&mut forced_all, "全点を使う（品質優先では固定）"),
                );
            } else {
                ui.checkbox(
                    &mut self.settings.convert.all_points,
                    "全点を使う（点数上限なし）",
                );
            }
            let custom_max = !quality && !self.settings.convert.all_points;
            ui.add_enabled_ui(custom_max, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("点数上限");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.convert.max_points)
                            .range(0..=usize::MAX)
                            .speed(100.0)
                            .suffix(" 点"),
                    );
                    ui.small("0でも全点。500,000点の上限はありません。");
                });
            });

            ui.separator();
            ui.strong("配置");
            ui.radio_value(
                &mut self.settings.convert.fit_mode,
                FitMode::Normalized,
                FitMode::Normalized.label(),
            );
            let normalized = self.settings.convert.fit_mode == FitMode::Normalized;
            ui.add_enabled_ui(normalized, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("最大辺");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.convert.fit_size)
                            .range(1.0..=1_000_000.0)
                            .speed(1.0),
                    );
                    ui.small("シーンの最大辺をこの値に合わせます。");
                });
            });
            ui.radio_value(
                &mut self.settings.convert.fit_mode,
                FitMode::Original,
                FitMode::Original.label(),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label("スケール");
                ui.add(
                    egui::DragValue::new(&mut self.settings.convert.scale)
                        .range(0.001..=1_000.0)
                        .speed(0.01),
                );
                ui.checkbox(&mut self.settings.convert.center, "中心を原点に合わせる");
            });
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.settings.convert.flip_y, "Y反転");
                ui.checkbox(&mut self.settings.convert.flip_z, "Z反転");
            });

            ui.separator();
            ui.strong("色と球面調和");
            if quality {
                let mut no_dither = false;
                ui.add_enabled(
                    false,
                    egui::Checkbox::new(&mut no_dither, "ディザ（品質優先ではオフ）"),
                );
                ui.checkbox(&mut self.settings.convert.keep_sh, "SH（視点依存の色）を保持");
                if !self.settings.convert.keep_sh {
                    ui.colored_label(egui::Color32::YELLOW, "SHを省略します。視点依存の色は保持されません。");
                }
            } else {
                ui.checkbox(&mut self.settings.convert.dither, "ディザ");
                ui.small("軽量モードはDC色のみです。SHを保持するには品質優先を選んでください。");
            }
        });
    }

    fn draw_status(&self, ui: &mut egui::Ui) {
        if self.job.is_some() {
            let elapsed = self
                .started_at
                .map(|started| started.elapsed().as_secs_f32())
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(16.0));
                ui.label(format!("変換中… {elapsed:.1} 秒"));
            });
            ui.small("進捗率は取得できません。完了まで待ってください。");
        }
        let color = match self.status_kind {
            StatusKind::Error => Some(egui::Color32::LIGHT_RED),
            StatusKind::Complete => Some(egui::Color32::from_rgb(120, 200, 140)),
            _ => None,
        };
        let text = egui::RichText::new(&self.status_text);
        let text = if let Some(color) = color {
            text.color(color)
        } else {
            text
        };
        ui.label(text);
    }

    fn draw_render_settings(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("表示設定（選択中の3DGS_Su）")
            .default_open(true)
            .show(ui, |ui| {
                ui.small(
                    "変更は「選択中へ適用」を押したときだけ反映され、アニメーションを保持します。",
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.button("選択中から読込").clicked() {
                        self.read_render_settings();
                    }
                    if ui.button("選択中へ適用").clicked() {
                        self.apply_render_settings();
                    }
                });
                let render = &mut self.settings.render;
                ui.horizontal_wrapped(|ui| {
                    ui.label("サイズ");
                    ui.add(
                        egui::DragValue::new(&mut render.size)
                            .range(0.01..=10.0)
                            .speed(0.01),
                    );
                    ui.label("不透明度");
                    ui.add(
                        egui::DragValue::new(&mut render.alpha)
                            .range(0.0..=1.0)
                            .speed(0.01),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("手前クリップ");
                    ui.add(
                        egui::DragValue::new(&mut render.near)
                            .range(0.1..=100_000.0)
                            .speed(0.1),
                    );
                    ui.label("表示上限（0=全点）");
                    ui.add(
                        egui::DragValue::new(&mut render.limit)
                            .range(0..=usize::MAX)
                            .speed(100.0)
                            .suffix(" 点"),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut render.sort, "奥から描く");
                    ui.checkbox(&mut render.ellipse, "楕円");
                    ui.checkbox(&mut render.panorama, "パノラマ");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("画角");
                    ui.add(
                        egui::DragValue::new(&mut render.fov)
                            .range(10.0..=170.0)
                            .speed(1.0),
                    );
                    egui::ComboBox::from_label("合成")
                        .selected_text(render.blend.label())
                        .show_ui(ui, |ui| {
                            for blend in BlendMode::ALL {
                                ui.selectable_value(&mut render.blend, blend, blend.label());
                            }
                        });
                });
            });
    }

    fn draw_overwrite_modal(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_overwrite.as_ref() else {
            return;
        };
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("上書き確認")
            .collapsible(false)
            .resizable(false)
            .default_pos(ctx.content_rect().center())
            .show(ctx, |ui| {
                ui.label("次のファイルが既にあります。上書きして変換しますか？");
                for path in &pending.existing {
                    ui.small(path.display().to_string());
                }
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("上書きして変換").clicked() {
                        confirm = true;
                    }
                    if ui.button("キャンセル").clicked() {
                        cancel = true;
                    }
                });
            });
        if confirm {
            if let Some(pending) = self.pending_overwrite.take() {
                self.begin_conversion(pending);
            }
        } else if cancel {
            self.pending_overwrite = None;
            self.set_status(StatusKind::Idle, "変換をキャンセルしました。");
        }
    }
}

impl eframe::App for ConverterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

impl Drop for ConverterApp {
    fn drop(&mut self) {
        if let Some(mut job) = self.job.take() {
            job.join();
        }
    }
}

fn ensure_su3ds(mut path: PathBuf) -> PathBuf {
    let is_su3ds = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("su3ds"));
    if !is_su3ds {
        path.set_extension("su3ds");
    }
    path
}

/// 出力パスを `.su3ds` に正規化する。自動検証からも利用できる。
pub fn normalized_su3ds_path(path: impl AsRef<Path>) -> PathBuf {
    ensure_su3ds(path.as_ref().to_path_buf())
}

/// 出力時に上書き確認が必要な既存ファイル（`.su3ds` と同名 `.json`）。
pub fn existing_outputs(output: &Path) -> Vec<PathBuf> {
    let json = output.with_extension("json");
    let mut paths = vec![output.to_path_buf()];
    if json != output {
        paths.push(json);
    }
    paths.retain(|path| path.is_file());
    paths
}

/// 出力時に上書き確認が必要な既存ファイル（公開ヘルパー）。
pub fn output_conflicts(output: impl AsRef<Path>) -> Vec<PathBuf> {
    existing_outputs(output.as_ref())
}

#[cfg(feature = "selftest")]
#[derive(Default)]
struct SelftestState {
    loaded: bool,
    active: bool,
    started: bool,
    status_cache: String,
    render_tested: bool,
}

#[cfg(feature = "selftest")]
struct SelftestConfig {
    input: PathBuf,
    output: Option<PathBuf>,
    settings: Settings,
}

#[cfg(feature = "selftest")]
impl ConverterApp {
    fn selftest_tick(&mut self) {
        if !self.selftest.loaded {
            self.selftest.loaded = true;
            match Self::load_selftest_config() {
                Ok(Some(config)) => {
                    self.selftest.active = true;
                    self.persist_settings = false;
                    self.input = config.input.to_string_lossy().into_owned();
                    self.output = config
                        .output
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_else(|| {
                            PathBuf::from(&self.input)
                                .with_extension("su3ds")
                                .to_string_lossy()
                                .into_owned()
                        });
                    self.output_auto = false;
                    self.settings = config.settings;
                    self.saved_settings = self.settings.clone();
                    self.set_status(StatusKind::Idle, "セルフテスト設定を読み込みました。");
                }
                Ok(None) => {}
                Err(error) => {
                    self.set_status(
                        StatusKind::Error,
                        format!("セルフテスト設定を読めません: {error}"),
                    );
                }
            }
        }

        if self.selftest.active
            && self.selftest.started
            && self.job.is_none()
            && self.status_kind == StatusKind::Complete
            && !self.selftest.render_tested
            && std::env::var_os("THREEDGS_SELFTEST_APPLY_RENDER").is_some()
        {
            self.selftest.render_tested = true;
            self.apply_render_settings();
        }
        if !self.selftest.active || self.selftest.started || self.job.is_some() {
            self.selftest_write_status();
            return;
        }
        match capture_targets() {
            Ok(targets) if !targets.is_empty() => {
                self.selftest.started = true;
                let output = match self.normalized_output() {
                    Ok(path) => path,
                    Err(error) => {
                        self.set_status(StatusKind::Error, error);
                        self.selftest_write_status();
                        return;
                    }
                };
                let pending = PendingConversion {
                    input: PathBuf::from(self.input.trim()),
                    output,
                    options: self.settings.convert.to_convert_options(),
                    targets,
                    epoch: current_epoch(),
                    quality: self.settings.convert.quality.is_quality(),
                    auto_apply: self.settings.auto_apply,
                    existing: Vec::new(),
                };
                self.begin_conversion(pending);
            }
            Ok(_) => {
                if self.status_kind != StatusKind::Busy {
                    self.set_status(
                        StatusKind::Idle,
                        "セルフテスト: 選択中の 3DGS_Su を待っています。",
                    );
                }
            }
            Err(error) => {
                self.set_status(
                    StatusKind::Error,
                    format!("セルフテスト: 選択オブジェクトを取得できません: {error}"),
                );
            }
        }
        self.selftest_write_status();
    }

    fn load_selftest_config() -> Result<Option<SelftestConfig>, String> {
        let Some(path) = std::env::var_os("THREEDGS_SELFTEST_CONFIG") else {
            return Ok(None);
        };
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", Path::new(&path).display()))?;
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        let input = value
            .get("input")
            .and_then(serde_json::Value::as_str)
            .filter(|input| !input.trim().is_empty())
            .ok_or_else(|| "input がありません".to_string())?;
        let output = value
            .get("output")
            .and_then(serde_json::Value::as_str)
            .filter(|output| !output.trim().is_empty())
            .map(PathBuf::from);

        let settings_value = value
            .get("settings")
            .cloned()
            .unwrap_or_else(|| value.clone());
        let mut settings: Settings =
            serde_json::from_value(settings_value).map_err(|error| format!("settings: {error}"))?;
        if value.get("settings").is_some()
            && let Some(auto_apply) = value.get("auto_apply").and_then(serde_json::Value::as_bool)
        {
            settings.auto_apply = auto_apply;
        }
        if let Some(preserve_quality) = value
            .get("preserve_quality")
            .and_then(serde_json::Value::as_bool)
        {
            settings.convert.quality = if preserve_quality {
                QualityPreset::Quality
            } else {
                QualityPreset::Lightweight
            };
        }
        Ok(Some(SelftestConfig {
            input: PathBuf::from(input),
            output,
            settings,
        }))
    }

    fn selftest_write_status(&mut self) {
        let Some(path) = std::env::var_os("THREEDGS_SELFTEST_STATUS") else {
            return;
        };
        let info = self.last_info.as_ref().map(|info| {
            serde_json::json!({
                "points_in_source": info.points_in_source,
                "points_written": info.points_written,
                "format": info.format,
                "sh_bands": info.sh_bands,
                "load_ms": info.load_ms,
                "process_ms": info.process_ms,
                "elapsed_ms": info.elapsed_ms,
            })
        });
        let diagnostic = serde_json::json!({
            "status": self.status_kind.key(),
            "busy": self.job.is_some(),
            "status_text": self.status_text,
            "input": self.input,
            "output": self.output,
            "applied": self.last_applied,
            "info": info,
            "settings": self.settings,
            "labels": {
                "quality": self.settings.convert.quality.label(),
                "fit": self.settings.convert.fit_mode.label(),
                "blend": self.settings.render.blend.label(),
            },
        });
        let Ok(text) = serde_json::to_string_pretty(&diagnostic) else {
            return;
        };
        if self.selftest.status_cache == text {
            return;
        }
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return;
        }
        if std::fs::write(&path, &text).is_ok() {
            self.selftest.status_cache = text;
        }
    }
}
