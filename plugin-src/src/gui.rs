//! 3DGS変換の専用UI。
//!
//! メニューを辿らず、このウィンドウだけで
//! 「ファイル選択 → .su3ds 変換 → 選択オブジェクトへ適用」まで完結させる。

use std::path::PathBuf;

use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};

use crate::{
    ConvertOptions, apply_to_selection, convert_file, last_file_path, pick_file,
    selected_count,
};

pub struct ConverterApp {
    _handle: AviUtl2EframeHandle,
    input: String,
    output: String,
    max_points: usize,
    auto_apply: bool,
    status: String,
    error: bool,
}

impl ConverterApp {
    pub fn new(cc: &eframe::CreationContext<'_>, handle: AviUtl2EframeHandle) -> Self {
        cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());
        let input = std::env::var("THREEDGS_TEST_INPUT").unwrap_or_default();
        let output = if input.is_empty() {
            String::new()
        } else {
            PathBuf::from(&input).with_extension("su3ds").to_string_lossy().into_owned()
        };
        Self {
            _handle: handle,
            input,
            output,
            max_points: 30_000,
            auto_apply: true,
            status: "3DGS_Su オブジェクトを選択してから使います。".into(),
            error: false,
        }
    }

    fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = text.into();
        self.error = error;
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
    }

    fn choose_input(&mut self) {
        match pick_file("3DGSを変換", &["sog", "ply"]) {
            Ok(Some(path)) => {
                self.input = path.to_string_lossy().into_owned();
                self.sync_output();
                self.set_status("入力ファイルを選びました。", false);
            }
            Ok(None) => {}
            Err(error) => self.set_status(format!("ファイルを選べません: {error}"), true),
        }
    }

    fn convert_and_apply(&mut self) {
        let input = PathBuf::from(self.input.trim());
        if !input.is_file() {
            self.set_status("入力ファイルが見つかりません。", true);
            return;
        }
        let output = input.with_extension("su3ds");
        let mut options = ConvertOptions::default();
        options.max = self.max_points;
        self.output = output.to_string_lossy().into_owned();
        self.set_status("変換しています…", false);
        let info = match convert_file(&input, &output, &options) {
            Ok(info) => info,
            Err(error) => {
                self.set_status(format!("変換に失敗しました: {error}"), true);
                return;
            }
        };
        let _ = crate::write_last_file(&output);
        let applied = if self.auto_apply {
            match apply_to_selection(&output) {
                Ok(count) => count,
                Err(error) => {
                    self.set_status(format!("変換はできましたが適用に失敗しました: {error}"), true);
                    return;
                }
            }
        } else {
            0
        };
        let suffix = if applied > 0 {
            format!(" / {applied} 件に適用")
        } else if self.auto_apply {
            " / 選択中の 3DGS_Su はありません".to_string()
        } else {
            String::new()
        };
        self.set_status(
            format!(
                "完了: {} 点 → {} 点 / {} ms{}",
                info.points_in_source, info.points_written, info.elapsed_ms, suffix
            ),
            applied == 0 && self.auto_apply,
        );
    }

    fn apply_existing(&mut self) {
        match pick_file("変換済みの .su3ds を選択", &["su3ds"]) {
            Ok(Some(path)) => self.apply_path(&path),
            Ok(None) => {}
            Err(error) => self.set_status(format!("ファイルを選べません: {error}"), true),
        }
    }

    fn apply_path(&mut self, path: &std::path::Path) {
        match apply_to_selection(path) {
            Ok(0) => self.set_status(
                "3DGS_Su オブジェクトが選択されていません。",
                true,
            ),
            Ok(count) => {
                self.output = path.to_string_lossy().into_owned();
                self.set_status(format!("{} 件に適用しました。", count), false);
            }
            Err(error) => self.set_status(format!("適用に失敗しました: {error}"), true),
        }
    }

    fn apply_last(&mut self) {
        match last_file_path().filter(|path| path.is_file()) {
            Some(path) => self.apply_path(&path),
            None => self.set_status("直前の変換結果がありません。", true),
        }
    }
}

impl eframe::App for ConverterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(path) = ui
                .ctx()
                .input(|input| input.raw.dropped_files.iter().find_map(|file| file.path.clone()))
            {
                self.input = path.to_string_lossy().into_owned();
                self.sync_output();
                self.set_status("入力ファイルを選びました。", false);
            }
            ui.heading("3DGS変換");
            ui.label("3DGS_Su に入れる .su3ds を作り、選択中のオブジェクトへ適用します。");
            ui.add_space(4.0);

            let selected = selected_count().unwrap_or(0);
            if selected == 0 {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 170, 90),
                    "3DGS_Su オブジェクトを選択してください（タイムラインまたはオブジェクトリスト）。",
                );
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(120, 200, 140),
                    format!("選択中: {} 件", selected),
                );
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("入力");
                let response = ui.add_sized(
                    [ui.available_width() - 92.0, 22.0],
                    egui::TextEdit::singleline(&mut self.input),
                );
                if response.changed() {
                    self.sync_output();
                }
                if ui.button("参照...").clicked() {
                    self.choose_input();
                }
            });
            ui.horizontal(|ui| {
                ui.label("出力");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.output)
                        .desired_width(ui.available_width()),
                );
            });
            ui.horizontal(|ui| {
                ui.label("点数の上限");
                ui.add(egui::DragValue::new(&mut self.max_points).range(100..=500_000));
                ui.checkbox(&mut self.auto_apply, "変換後に選択中へ適用");
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("変換して適用").min_size(egui::vec2(130.0, 30.0)))
                    .clicked()
                {
                    self.convert_and_apply();
                }
                if ui.button("既存の .su3ds を適用...").clicked() {
                    self.apply_existing();
                }
                if ui.button("直前の結果を適用").clicked() {
                    self.apply_last();
                }
            });
            ui.separator();
            ui.label(if self.error {
                egui::RichText::new(&self.status).color(egui::Color32::LIGHT_RED)
            } else {
                egui::RichText::new(&self.status)
            });
            ui.add_space(8.0);
            ui.small("入力は .sog / .ply。出力は同じフォルダーの同名 .su3ds です。");
            ui.small("うまく表示されない場合は、AviUtl2 のオブジェクト設定でサイズと表示上限を調整してください。");
        });
    }
}
