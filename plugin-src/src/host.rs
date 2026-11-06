//! UIスレッドからだけ呼ぶ、対象の取得・検証・設定適用。
use crate::{EDIT_HANDLE, EFFECT, ITEM};
use aviutl2::{AnyResult, generic::ObjectHandle};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

pub(crate) static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static EPOCH: AtomicU64 = AtomicU64::new(0);
static TARGETS: Mutex<(u64, Vec<ObjectHandle>)> = Mutex::new((0, Vec::new()));
const ITEMS: &[&str] = &[
    "サイズ",
    "不透明度",
    "手前クリップ",
    "表示上限",
    "奥から描く",
    "楕円",
    "パノラマ",
    "画角",
    "合成",
];

pub(crate) fn current_epoch() -> u64 {
    EPOCH.load(Ordering::SeqCst)
}
pub(crate) fn invalidate_targets() {
    EPOCH.fetch_add(1, Ordering::SeqCst);
}
pub(crate) fn ready() -> bool {
    !SHUTDOWN.load(Ordering::SeqCst) && EDIT_HANDLE.is_ready()
}

pub(crate) fn capture_targets() -> AnyResult<Vec<ObjectHandle>> {
    if !ready() {
        return Ok(Vec::new());
    }
    let epoch = current_epoch();
    let previous = TARGETS
        .lock()
        .map(|s| {
            if s.0 == epoch {
                s.1.clone()
            } else {
                Vec::new()
            }
        })
        .unwrap_or_default();
    // 選択状態のポーリングは読取区間で完結させる。
    // 編集区間を定期要求すると、描画・終了時のホスト編集キューと競合する。
    let objects = EDIT_HANDLE.call_read_section(move |section| {
        let mut objects = section.get_selected_objects().unwrap_or_default();
        if objects.is_empty()
            && let Ok(Some(object)) = section.get_focused_object()
        {
            objects.push(object);
        }
        // 明示的に別のオブジェクトを選んだ時は過去の対象へフォールバックしない。
        if objects.is_empty() {
            objects = previous;
        }
        objects.retain(|object| {
            section
                .get_object_effect_item(*object, EFFECT, 0, ITEM)
                .is_ok()
        });
        objects
    })?;
    if let Ok(mut state) = TARGETS.lock() {
        *state = (epoch, objects.clone());
    }
    Ok(objects)
}

pub(crate) fn apply_properties(
    targets: Vec<ObjectHandle>,
    properties: Vec<(String, String)>,
) -> AnyResult<usize> {
    if !ready() {
        return Err(std::io::Error::other("AviUtl2が操作可能な状態ではありません").into());
    }
    for (item, value) in &properties {
        if value.contains(['\r', '\n']) {
            return Err(std::io::Error::other(format!("{item}: 改行は使えません")).into());
        }
    }
    let properties = refresh_revision(properties);
    let result = EDIT_HANDLE.call_edit_section(move |section| -> Result<usize, String> {
        // 書込前に全対象と全項目の存在を確認。削除済みハンドルを使わない。
        for object in &targets {
            for (item, _) in &properties {
                section
                    .get_object_effect_item(*object, EFFECT, 0, item)
                    .map_err(|e| format!("対象または項目がありません: {item}: {e}"))?;
            }
        }
        for object in &targets {
            for (item, value) in &properties {
                section
                    .set_object_effect_item(*object, EFFECT, 0, item, value)
                    .map_err(|e| format!("設定できません: {item}: {e}"))?;
                let read = section
                    .get_object_effect_item(*object, EFFECT, 0, item)
                    .map_err(|e| format!("読戻しに失敗: {item}: {e}"))?;
                if !same_value(&read, value) {
                    return Err(format!("設定値が一致しません: {item}: {read}"));
                }
            }
        }
        Ok(targets.len())
    })?;
    result.map_err(|e| std::io::Error::other(e).into())
}

fn refresh_revision(mut properties: Vec<(String, String)>) -> Vec<(String, String)> {
    static REVISION: AtomicU64 = AtomicU64::new(0);
    if properties.iter().any(|(key, _)| key == ITEM) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let previous = REVISION
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                Some(old.max(now) + 1)
            })
            .unwrap();
        properties.retain(|(key, _)| key != "更新番号");
        properties.push(("更新番号".into(), (previous.max(now) + 1).to_string()));
    }
    properties
}

fn same_value(read: &str, expected: &str) -> bool {
    if read == expected {
        return true;
    }
    match (read.parse::<f64>(), expected.parse::<f64>()) {
        (Ok(a), Ok(b)) => a.is_finite() && b.is_finite() && (a - b).abs() <= 0.00001,
        _ => false,
    }
}

pub(crate) fn read_properties() -> AnyResult<Vec<(String, String)>> {
    let targets = capture_targets()?;
    let Some(object) = targets.first().copied() else {
        return Err(std::io::Error::other("3DGS_Suを選択してください").into());
    };
    let result = EDIT_HANDLE.call_read_section(move |section| {
        ITEMS
            .iter()
            .map(|item| {
                section
                    .get_object_effect_item(object, EFFECT, 0, item)
                    .map(|v| ((*item).to_string(), v))
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()
    })?;
    result.map_err(|e| std::io::Error::other(e).into())
}

#[cfg(test)]
mod tests {
    use super::{refresh_revision, same_value};
    #[test]
    fn file_application_gets_a_fresh_revision() {
        let a = refresh_revision(vec![(crate::ITEM.into(), "sample.su3ds".into())]);
        let b = refresh_revision(vec![(crate::ITEM.into(), "sample.su3ds".into())]);
        assert_eq!(a.len(), 2);
        assert_ne!(a[1].1, b[1].1);
        assert_eq!(a[1].0, "更新番号");
        assert_eq!(
            refresh_revision(vec![("サイズ".into(), "1".into())]).len(),
            1
        );
    }
    #[test]
    fn readback_values() {
        assert!(same_value("1.000", "1"));
        assert!(!same_value("1,2,直線移動", "1"));
        assert!(same_value("通常", "通常"));
        assert!(!same_value("NaN", "0"));
    }
}
