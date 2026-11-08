//! 3DGS_Su の変換を AviUtl2 のメニューから行う汎用プラグイン。
//!
//! `.sog` / `.ply` を選ぶと同じ場所に `.su3ds` を作り、
//! 選択中の 3DGS_Su オブジェクトへ適用する。
//! 変換はプラグイン内の Rust 実装で行い、外部プロセスを使わない。

pub mod convert;
pub mod gui;
mod host;
pub mod settings;
pub(crate) use host::{apply_properties, capture_targets, current_epoch, read_properties};

use std::fs;
use std::path::{Path, PathBuf};

use aviutl2::generic::GlobalEditHandle;
use aviutl2::{AnyResult, generic::GenericPlugin};
use aviutl2_eframe::EframeWindow;
pub use convert::{ConvertInfo, ConvertOptions, convert_file};

const EFFECT: &str = "3DGS_Su";
const ITEM: &str = "3DGSファイル(.su3ds)";
const PLUGIN_DIR: &str = "3DGSConverter_Su";
const LAST_FILE: &str = "last_converted.txt";

pub(crate) static EDIT_HANDLE: GlobalEditHandle = GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
pub struct ThreeDgsConverterSu {
    window: Option<EframeWindow>,
}

impl GenericPlugin for ThreeDgsConverterSu {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        Self::init_logging();
        tracing::info!("Initializing 3DGSConverter_Su");
        let window = EframeWindow::new("3DGS変換", |cc, handle| {
            Ok(Box::new(gui::ConverterApp::new(cc, handle)))
        })?;

        Ok(Self {
            window: Some(window),
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "3DGSConverter_Su".to_string(),
            information: format!(
                "3DGS (.sog/.ply) -> .su3ds converter for AviUtl2 / v{}",
                env!("CARGO_PKG_VERSION")
            ),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        EDIT_HANDLE.init(registry.create_edit_handle());
        if let Some(window) = &self.window {
            match window.handle() {
                Ok(handle) => {
                    if let Err(error) = registry.register_window_client("3DGS変換", &handle) {
                        tracing::error!("3DGS変換ウィンドウの登録に失敗: {error}");
                    }
                }
                Err(error) => tracing::error!("3DGS変換ウィンドウの初期化に失敗: {error}"),
            }
        }
        // 配置・表示・ドッキングはAviUtl2の標準「表示」メニューに任せる。
        // 子HWNDを独自に移動/拡大すると、ホストの他パネルを覆ってしまう。
        tracing::info!("3DGSConverter_Su UI registered");
    }

    fn on_project_load(&mut self, _project: &mut aviutl2::generic::ProjectFile) {
        host::invalidate_targets();
    }

    fn event_change_scene_info(&mut self) {
        host::invalidate_targets();
    }
}

impl Drop for ThreeDgsConverterSu {
    fn drop(&mut self) {
        host::SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
        // UIと変換ワーカーが終了してからDLLをアンロードする。
        drop(self.window.take());
    }
}

impl ThreeDgsConverterSu {
    fn init_logging() {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();
    }
}

aviutl2::register_generic_plugin!(ThreeDgsConverterSu);

pub(crate) fn plugin_data_dir() -> PathBuf {
    std::env::var_os("THREEDGS_SETTINGS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(PLUGIN_DIR)
        })
}

pub(crate) fn last_file_path() -> Option<PathBuf> {
    fs::read_to_string(plugin_data_dir().join(LAST_FILE))
        .ok()
        .map(|text| PathBuf::from(text.trim()))
        .filter(|path| !path.as_os_str().is_empty())
}

pub(crate) fn write_last_file(path: &Path) -> AnyResult<()> {
    let last = plugin_data_dir().join(LAST_FILE);
    if let Some(parent) = last.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&last, path.to_string_lossy().as_bytes())?;
    Ok(())
}

pub(crate) fn selected_count() -> AnyResult<usize> {
    Ok(capture_targets()?.len())
}

pub(crate) fn pick_file(title: &str, extensions: &[&str]) -> AnyResult<Option<PathBuf>> {
    let selected = native_dialog::FileDialogBuilder::default()
        .set_title(title)
        .add_filter("対象ファイル", extensions)
        .open_single_file()
        .show()?;
    Ok(selected)
}
