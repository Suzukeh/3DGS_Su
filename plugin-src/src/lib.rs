//! 3DGS_Su の変換を AviUtl2 のメニューから行う汎用プラグイン。
//!
//! `.sog` / `.ply` を選ぶと同じ場所に `.su3ds` を作り、
//! 選択中の 3DGS_Su オブジェクトへ適用する。
//! 変換はプラグイン内の Rust 実装で行い、外部プロセスを使わない。

pub mod convert;
mod gui;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use aviutl2::config::app_data_path;
use aviutl2::generic::{GlobalEditHandle, ObjectHandle};
use aviutl2::{AnyResult, generic::GenericPlugin};
use aviutl2_eframe::EframeWindow;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
pub use convert::{ConvertInfo, ConvertOptions, convert_file};

const EFFECT: &str = "3DGS_Su";
const ITEM: &str = "3DGSファイル(.su3ds)";
const PLUGIN_DIR: &str = "3DGSConverter_Su";
const LAST_FILE: &str = "last_converted.txt";

pub(crate) static EDIT_HANDLE: GlobalEditHandle = GlobalEditHandle::new();
static WINDOW_HWND: OnceLock<isize> = OnceLock::new();
static TARGETS: Mutex<Vec<ObjectHandle>> = Mutex::new(Vec::new());

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
                    if handle
                        .window_handle()
                        .ok()
                        .and_then(|raw| match raw.as_raw() {
                            RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
                            _ => None,
                        })
                        .is_some_and(|hwnd| WINDOW_HWND.set(hwnd).is_ok())
                    {
                        tracing::info!("3DGS変換ウィンドウを登録");
                    }
                    if let Err(error) = registry.register_window_client("3DGS変換", &handle) {
                        tracing::error!("3DGS変換ウィンドウの登録に失敗: {error}");
                    }
                }
                Err(error) => tracing::error!("3DGS変換ウィンドウの初期化に失敗: {error}"),
            }
        }
        registry.register_object_menu("3DGS変換...", show_window);
        registry.register_edit_menu("3DGS変換...", show_window);
        show_window();
        tracing::info!("3DGSConverter_Su UI registered");
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
    app_data_path().join("Plugin").join(PLUGIN_DIR)
}

/// 登録済みの専用UIを前面に出す。AviUtl2 のメニューから呼ばれる。
pub(crate) fn show_window() {
    capture_selection();
    if let Some(hwnd) = WINDOW_HWND.get() {
        // HWND は EframeWindow が生存している間だけ有効。
        unsafe {
            let handle = *hwnd as *mut std::ffi::c_void;
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                handle,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW,
            );
            let parent = windows_sys::Win32::UI::WindowsAndMessaging::GetParent(handle);
            let mut client = windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if !parent.is_null() {
                windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(parent, &mut client);
            }
            let width = 760;
            let height = 280;
            let y = (client.bottom - height - 20).max(0);
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                handle,
                std::ptr::null_mut(),
                20,
                y,
                width,
                height,
                windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                    | windows_sys::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW,
            );
            windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(handle);
        }
    }
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

/// 選択が UI クリックで外れる前に、対象オブジェクトを覚えておく。
pub(crate) fn capture_selection() {
    if !EDIT_HANDLE.is_ready() {
        return;
    }
    let objects = EDIT_HANDLE
        .call_edit_section(|section| {
            let mut objects = section.get_selected_objects().unwrap_or_default();
            if objects.is_empty()
                && let Ok(Some(object)) = section.get_focused_object()
            {
                objects.push(object);
            }
            objects
        })
        .unwrap_or_default();
    if objects.is_empty() {
        return;
    }
    if let Ok(mut targets) = TARGETS.lock() {
        *targets = objects;
    }
}

pub(crate) fn selected_count() -> AnyResult<usize> {
    if !EDIT_HANDLE.is_ready() {
        return Ok(0);
    }
    capture_selection();
    let fallback = TARGETS.lock().map(|targets| targets.len()).unwrap_or(0);
    Ok(EDIT_HANDLE.call_edit_section(|section| {
        let objects = section.get_selected_objects().unwrap_or_default();
        if !objects.is_empty() {
            return objects.len();
        }
        if section.get_focused_object().ok().flatten().is_some() {
            return 1;
        }
        fallback
    })?)
}

pub(crate) fn pick_file(title: &str, extensions: &[&str]) -> AnyResult<Option<PathBuf>> {
    let selected = native_dialog::FileDialogBuilder::default()
        .set_title(title)
        .add_filter("対象ファイル", extensions)
        .open_single_file()
        .show()?;
    Ok(selected)
}

pub(crate) fn apply_to_selection(path: &Path) -> AnyResult<usize> {
    let value = path.to_string_lossy().to_string();
    let fallback = TARGETS.lock().map(|targets| targets.clone()).unwrap_or_default();
    let applied = EDIT_HANDLE.call_edit_section(move |section| {
        let mut applied = 0usize;
        let mut objects = section.get_selected_objects().unwrap_or_default();
        if objects.is_empty()
            && let Ok(Some(object)) = section.get_focused_object()
        {
            objects.push(object);
        }
        if objects.is_empty() {
            objects = fallback;
        }
        for object in objects {
            if section
                .set_object_effect_item(object, EFFECT, 0, ITEM, &value)
                .is_ok()
            {
                applied += 1;
            }
        }
        applied
    })?;
    tracing::info!("applied {:?} to {} object(s)", path, applied);
    Ok(applied)
}
