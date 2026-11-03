//! 3DGS_Su の変換を AviUtl2 のメニューから行う汎用プラグイン。
//!
//! `.sog` / `.ply` を選ぶと同じ場所に `.su3ds` を作り、
//! 選択中の 3DGS_Su オブジェクトへ適用する。
//! 変換はプラグイン内の Rust 実装で行い、外部プロセスを使わない。

pub mod convert;

use std::fs;
use std::path::{Path, PathBuf};

use aviutl2::config::app_data_path;
use aviutl2::generic::GlobalEditHandle;
use aviutl2::{AnyResult, generic::GenericPlugin};

const EFFECT: &str = "3DGS_Su";
const ITEM: &str = "3DGSファイル(.su3ds)";
const PLUGIN_DIR: &str = "3DGSConverter_Su";
const LAST_FILE: &str = "last_converted.txt";

pub(crate) static EDIT_HANDLE: GlobalEditHandle = GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
pub struct ThreeDgsConverterSu;

impl GenericPlugin for ThreeDgsConverterSu {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        Self::init_logging();
        tracing::info!("Initializing 3DGSConverter_Su");

        Ok(Self)
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
        registry.register_menus::<Self>();
        tracing::info!("3DGSConverter_Su menus registered");
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

#[aviutl2::generic::menus]
impl ThreeDgsConverterSu {
    #[edit(name = "3DGS変換\\3DGSを変換して適用...")]
    fn convert_and_apply(&mut self) -> AnyResult<()> {
        if !has_selection()? {
            return message(
                "3DGS変換",
                "3DGS_Su オブジェクトを選択してから実行してください。",
            );
        }
        // 検証用: THREEDGS_TEST_INPUT があればファイルダイアログを省く
        let input = match std::env::var("THREEDGS_TEST_INPUT") {
            Ok(path) if !path.is_empty() => PathBuf::from(path),
            _ => match pick_file("3DGSを変換", &["sog", "ply"])? {
                Some(path) => path,
                None => return Ok(()),
            },
        };

        let output = input.with_extension("su3ds");
        let mut options = convert::ConvertOptions::default();
        if std::env::var_os("THREEDGS_TEST_NO_DITHER").is_some() {
            options.no_dither = true;
        }
        let info = match convert::convert_file(&input, &output, &options) {
            Ok(info) => info,
            Err(error) => {
                return message(
                    "3DGS変換",
                    format!("変換に失敗しました。\n\n{error}"),
                );
            }
        };
        tracing::info!(
            "converted {} -> {} ({} -> {} points, {} ms)",
            input.display(),
            output.display(),
            info.points_in_source,
            info.points_written,
            info.elapsed_ms
        );
        tracing::info!("format={} sh_bands={:?}", info.format, info.sh_bands);
        let last = plugin_data_dir().join(LAST_FILE);
        if let Some(parent) = last.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&last, output.to_string_lossy().as_bytes())?;
        let applied = apply_to_selection(&output)?;
        let text = if applied > 0 {
            format!(
                "変換しました。\n{}\n{} 点 → {} 点 / {} ms\n\n3DGS_Su オブジェクト {} 件に適用しました。",
                output.display(),
                info.points_in_source,
                info.points_written,
                info.elapsed_ms,
                applied
            )
        } else {
            format!(
                "変換しました。\n{}\n{} 点 → {} 点 / {} ms\n\n3DGS_Su オブジェクトが選択されていないため、適用はしていません。",
                output.display(),
                info.points_in_source,
                info.points_written,
                info.elapsed_ms
            )
        };
        message("3DGS変換", text)
    }

    #[edit(name = "3DGS変換\\既存の .su3ds を適用...")]
    fn apply_existing(&mut self) -> AnyResult<()> {
        if !has_selection()? {
            return message(
                "3DGS変換",
                "3DGS_Su オブジェクトを選択してから実行してください。",
            );
        }
        let Some(path) = pick_file("変換済みの .su3ds を選択", &["su3ds"])? else {
            return Ok(());
        };
        let applied = apply_to_selection(&path)?;
        let text = if applied > 0 {
            format!(
                "{}\n\n3DGS_Su オブジェクト {} 件に適用しました。",
                path.display(),
                applied
            )
        } else {
            "3DGS_Su オブジェクトが選択されていません。\nオブジェクトを選択してから実行してください。"
                .to_string()
        };
        message("3DGS変換", text)
    }

    #[edit(name = "3DGS変換\\直前の変換結果を適用")]
    fn apply_last(&mut self) -> AnyResult<()> {
        let last = plugin_data_dir().join(LAST_FILE);
        let path = match fs::read_to_string(&last) {
            Ok(text) => PathBuf::from(text.trim()),
            Err(_) => {
                return message(
                    "3DGS変換",
                    "直前の変換結果がありません。先に「3DGSを変換して適用...」を実行してください。",
                );
            }
        };
        if !path.is_file() {
            return message(
                "3DGS変換",
                format!("ファイルが見つかりません。\n{}", path.display()),
            );
        }
        let applied = apply_to_selection(&path)?;
        let text = if applied > 0 {
            format!(
                "{}\n\n3DGS_Su オブジェクト {} 件に適用しました。",
                path.display(),
                applied
            )
        } else {
            format!(
                "{}\n\n3DGS_Su オブジェクトが選択されていません。\nオブジェクトを選択してから実行してください。",
                path.display()
            )
        };
        message("3DGS変換", text)
    }

}

aviutl2::register_generic_plugin!(ThreeDgsConverterSu);

fn plugin_data_dir() -> PathBuf {
    app_data_path().join("Plugin").join(PLUGIN_DIR)
}

fn pick_file(title: &str, extensions: &[&str]) -> AnyResult<Option<PathBuf>> {
    let selected = native_dialog::FileDialogBuilder::default()
        .set_title(title)
        .add_filter("対象ファイル", extensions)
        .open_single_file()
        .show()?;
    Ok(selected)
}

fn message(title: &str, text: impl ToString) -> AnyResult<()> {
    native_dialog::DialogBuilder::message()
        .set_title(title)
        .set_text(text.to_string())
        .alert()
        .show()?;
    Ok(())
}

fn has_selection() -> AnyResult<bool> {
    let has = EDIT_HANDLE.call_edit_section(|section| {
        section
            .get_selected_objects()
            .map(|objects| !objects.is_empty())
            .unwrap_or(false)
    })?;
    Ok(has)
}

fn apply_to_selection(path: &Path) -> AnyResult<usize> {
    let value = path.to_string_lossy().to_string();
    let applied = EDIT_HANDLE.call_edit_section(move |section| {
        let mut applied = 0usize;
        match section.get_selected_objects() {
            Ok(objects) => {
                for object in objects {
                    if section
                        .set_object_effect_item(object, EFFECT, 0, ITEM, &value)
                        .is_ok()
                    {
                        applied += 1;
                    }
                }
            }
            Err(error) => tracing::warn!("get_selected_objects failed: {error:?}"),
        }
        applied
    })?;
    tracing::info!("applied {:?} to {} object(s)", path, applied);
    Ok(applied)
}
