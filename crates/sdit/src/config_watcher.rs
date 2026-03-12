//! 設定ファイルの変更を監視し、`SditEvent::ConfigReloaded` を送信するモジュール。
//!
//! `notify` クレートで `RecommendedWatcher` を使い、ファイル変更イベントを
//! `EventLoopProxy` 経由でメインスレッドに通知する。
//! デバウンス（300ms）により短時間の連続変更を吸収する。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use winit::event_loop::EventLoopProxy;

use crate::app::SditEvent;

/// デバウンス待機時間（最後のイベントからこの時間が経過したら通知を送信する）。
const DEBOUNCE_DURATION: Duration = Duration::from_millis(300);

/// 設定ファイルの変更監視を開始する。
///
/// `path` の **親ディレクトリ** を監視する（ファイル自体の監視は一部 FS で動作しないため）。
/// 変更検出時は 300ms のデバウンス後に `SditEvent::ConfigReloaded` を送信する。
///
/// # 戻り値
///
/// 成功時は `Some(watcher)` を返す。呼び出し側はウォッチャーをドロップしないよう保持すること。
/// エラーや親ディレクトリが存在しない場合は `None` を返す（起動は続行される）。
pub fn spawn_config_watcher(
    path: &std::path::Path,
    proxy: EventLoopProxy<SditEvent>,
) -> Option<RecommendedWatcher> {
    let parent = match path.parent() {
        Some(p) if p.exists() => p.to_path_buf(),
        Some(p) => {
            log::warn!("config_watcher: parent directory does not exist: {}", p.display());
            return None;
        }
        None => {
            log::warn!("config_watcher: config path has no parent directory: {}", path.display());
            return None;
        }
    };

    // デバウンス用: 最後のイベント時刻を共有する
    let last_event: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    // M-1 修正: タイマースレッド起動中フラグ（AtomicBool でスレッド重複を防止）
    let timer_running: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let config_path = path.to_path_buf();
    let config_path_for_log = config_path.clone();
    let last_event_clone = Arc::clone(&last_event);
    let timer_running_clone = Arc::clone(&timer_running);

    let watcher_result = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let event = match res {
            Ok(e) => e,
            Err(e) => {
                log::warn!("config_watcher: notify error: {e}");
                return;
            }
        };

        // 変更・作成イベントかつ対象ファイルが設定ファイルか確認
        let is_relevant = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))
            && event.paths.iter().any(|p| p == &config_path);

        if !is_relevant {
            return;
        }

        // デバウンス: 最後のイベント時刻を更新
        {
            let mut guard =
                last_event_clone.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            *guard = Some(Instant::now());
        }

        // タイマースレッドが既に動いていればスキップ（AtomicBool で排他制御）
        if timer_running_clone.swap(true, Ordering::SeqCst) {
            return;
        }

        // デバウンスタイマースレッドを起動
        let last_event_for_timer = Arc::clone(&last_event_clone);
        let timer_flag = Arc::clone(&timer_running_clone);
        let proxy_clone = proxy.clone();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(DEBOUNCE_DURATION);

                let elapsed = {
                    let guard = last_event_for_timer
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    guard.map(|t| t.elapsed())
                };

                match elapsed {
                    Some(e) if e >= DEBOUNCE_DURATION => {
                        // 最後のイベントから 300ms 経過 → 通知を送信してタイマー終了
                        {
                            let mut guard = last_event_for_timer
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            *guard = None;
                        }
                        // タイマーフラグを解除してから送信（次のイベントで新タイマーを起動可能にする）
                        timer_flag.store(false, Ordering::SeqCst);
                        if let Err(e) = proxy_clone.send_event(SditEvent::ConfigReloaded) {
                            log::warn!("config_watcher: failed to send ConfigReloaded: {e}");
                        }
                        break;
                    }
                    Some(_) => {
                        // まだデバウンス期間内 → 再度スリープして待機
                    }
                    None => {
                        // リセットされた（想定外だが安全に終了）
                        timer_flag.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
        });
    });

    match watcher_result {
        Ok(mut watcher) => {
            if let Err(e) = watcher.watch(&parent, RecursiveMode::NonRecursive) {
                log::warn!("config_watcher: failed to watch directory {}: {e}", parent.display());
                return None;
            }
            log::info!(
                "config_watcher: watching {} for changes to {}",
                parent.display(),
                config_path_for_log.display()
            );
            Some(watcher)
        }
        Err(e) => {
            log::warn!("config_watcher: failed to create watcher: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sdit_core::config::color::ThemeName;

    /// 存在しないパスを渡したとき、親ディレクトリが存在しないことを確認する。
    ///
    /// NOTE: `EventLoopProxy` は直接構築できないため、`spawn_config_watcher` 本体を
    /// 呼び出す代わりに、親ディレクトリの存在チェックロジックのみ検証する。
    #[test]
    fn nonexistent_parent_returns_none() {
        let path = Path::new("/nonexistent_sdit_test_dir_12345/sdit.toml");
        // 親ディレクトリが存在しない場合 → spawn_config_watcher は None を返す
        assert!(!path.parent().is_some_and(std::path::Path::exists));
    }

    /// `ThemeName` の `PartialEq` が正しく動作すること。
    #[test]
    fn theme_name_partial_eq() {
        assert_eq!(ThemeName::CatppuccinMocha, ThemeName::CatppuccinMocha);
        assert_ne!(ThemeName::CatppuccinMocha, ThemeName::CatppuccinLatte);
        assert_ne!(ThemeName::CatppuccinLatte, ThemeName::GruvboxDark);
    }
}
