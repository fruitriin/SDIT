use super::*;

#[test]
fn scrollbar_config_default() {
    let cfg = ScrollbarConfig::default();
    assert!(cfg.enabled, "enabled のデフォルトは true");
    assert_eq!(cfg.width, 8, "width のデフォルトは 8");
}

#[test]
fn scrollbar_config_clamped_width() {
    let cfg = ScrollbarConfig { enabled: true, width: 1 };
    assert_eq!(cfg.clamped_width(), 2, "width=1 はクランプされて 2 になる");

    let cfg = ScrollbarConfig { enabled: true, width: 8 };
    assert_eq!(cfg.clamped_width(), 8, "width=8 はそのまま");

    let cfg = ScrollbarConfig { enabled: true, width: 33 };
    assert_eq!(cfg.clamped_width(), 32, "width=33 はクランプされて 32 になる");
}

#[test]
fn scrollbar_config_deserialize() {
    let toml_str = "[scrollbar]\nenabled = false\nwidth = 12\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert!(!cfg.scrollbar.enabled);
    assert_eq!(cfg.scrollbar.width, 12);
}

#[test]
fn scrollbar_config_default_in_full_config() {
    let cfg = Config::default();
    assert!(cfg.scrollbar.enabled);
    assert_eq!(cfg.scrollbar.width, 8);
}

#[test]
fn security_config_default() {
    let cfg = SecurityConfig::default();
    assert!(!cfg.auto_secure_input, "auto_secure_input のデフォルトは false");
}

#[test]
fn security_config_deserialize() {
    let toml_str = "[security]\nauto_secure_input = true\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert!(cfg.security.auto_secure_input);
}

#[test]
fn security_config_default_in_full_config() {
    let cfg = Config::default();
    assert!(!cfg.security.auto_secure_input);
}

#[test]
fn default_config_is_valid() {
    let config = Config::default();
    assert!(!config.font.family.is_empty());
    assert!(config.font.size > 0.0);
}

#[test]
fn option_as_alt_default_is_none() {
    let config = Config::default();
    assert_eq!(config.option_as_alt, OptionAsAlt::None);
}

#[test]
fn option_as_alt_deserialize_canonical() {
    let cases: &[(&str, OptionAsAlt)] = &[
        (r#"option_as_alt = "none""#, OptionAsAlt::None),
        (r#"option_as_alt = "both""#, OptionAsAlt::Both),
        (r#"option_as_alt = "only_left""#, OptionAsAlt::OnlyLeft),
        (r#"option_as_alt = "only_right""#, OptionAsAlt::OnlyRight),
    ];
    for (toml_str, expected) in cases {
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.option_as_alt, *expected, "failed for: {toml_str}");
    }
}

#[test]
fn option_as_alt_deserialize_alias() {
    let left: Config = toml::from_str(r#"option_as_alt = "left""#).unwrap();
    assert_eq!(left.option_as_alt, OptionAsAlt::OnlyLeft);

    let right: Config = toml::from_str(r#"option_as_alt = "right""#).unwrap();
    assert_eq!(right.option_as_alt, OptionAsAlt::OnlyRight);
}

#[test]
fn option_as_alt_serialize() {
    let config = Config { option_as_alt: OptionAsAlt::Both, ..Default::default() };
    let serialized = toml::to_string(&config).unwrap();
    assert!(
        serialized.contains("option_as_alt = \"both\""),
        "expected serialized form: {serialized}"
    );
}

#[test]
fn load_nonexistent_returns_default() {
    let config = Config::load(Path::new("/nonexistent/path/sdit.toml"));
    assert!(!config.font.family.is_empty());
    assert!((config.font.size - 14.0).abs() < f32::EPSILON);
}

#[test]
fn deserialize_full_config() {
    let toml_str = r#"
[font]
family = "JetBrains Mono"
size = 16.0
line_height = 1.3
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.font.family, "JetBrains Mono");
    assert!((config.font.size - 16.0).abs() < f32::EPSILON);
    assert!((config.font.line_height - 1.3).abs() < f32::EPSILON);
}

#[test]
fn deserialize_empty_uses_defaults() {
    let config: Config = toml::from_str("").unwrap();
    assert!(!config.font.family.is_empty());
    assert!((config.font.size - 14.0).abs() < f32::EPSILON);
}

#[test]
fn default_path_not_empty() {
    let path = Config::default_path();
    assert!(!path.as_os_str().is_empty());
}

#[test]
fn config_save_and_load_roundtrip() {
    let config = Config::default();
    let path = std::path::PathBuf::from("tmp/test-config-roundtrip.toml");
    std::fs::create_dir_all("tmp").expect("tmp dir");
    config.save(&path).expect("save failed");
    let loaded = Config::load(&path);
    assert!(
        (loaded.font.size - config.font.size).abs() < f32::EPSILON,
        "font.size mismatch: {} vs {}",
        loaded.font.size,
        config.font.size
    );
    assert_eq!(loaded.font.family, config.font.family);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bell_config_default() {
    let bell = BellConfig::default();
    assert!(bell.visual);
    assert!(bell.dock_bounce);
    assert_eq!(bell.duration_ms, 150);
}

#[test]
fn bell_config_deserialize() {
    let toml_str = "[bell]\nvisual = false\ndock_bounce = false\nduration_ms = 200\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.bell.visual);
    assert!(!config.bell.dock_bounce);
    assert_eq!(config.bell.duration_ms, 200);
}

#[test]
fn bell_config_partial_deserialize() {
    // 部分指定のとき残りはデフォルト補完
    let toml_str = "[bell]\nduration_ms = 300\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.bell.visual); // default: true
    assert!(config.bell.dock_bounce); // default: true
    assert_eq!(config.bell.duration_ms, 300);
}

#[test]
fn bell_duration_clamp_zero() {
    let bell = BellConfig { duration_ms: 0, ..Default::default() };
    assert_eq!(bell.clamped_duration_ms(), 1);
}

#[test]
fn bell_duration_clamp_max() {
    let bell = BellConfig { duration_ms: 999_999, ..Default::default() };
    assert_eq!(bell.clamped_duration_ms(), 5000);
}

#[test]
fn window_config_default() {
    let wc = WindowConfig::default();
    assert!((wc.opacity - 1.0).abs() < f32::EPSILON);
    assert!(!wc.blur);
    assert_eq!(wc.padding_x, 0);
    assert_eq!(wc.padding_y, 0);
}

#[test]
fn window_config_deserialize() {
    let toml_str = "[window]\nopacity = 0.8\nblur = true\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!((config.window.opacity - 0.8).abs() < f32::EPSILON);
    assert!(config.window.blur);
}

#[test]
fn window_config_padding_deserialize() {
    let toml_str = "[window]\npadding_x = 8\npadding_y = 4\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.padding_x, 8);
    assert_eq!(config.window.padding_y, 4);
}

#[test]
fn window_config_partial_deserialize() {
    let toml_str = "[window]\nopacity = 0.5\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!((config.window.opacity - 0.5).abs() < f32::EPSILON);
    assert!(!config.window.blur); // default
}

#[test]
fn window_opacity_clamp() {
    let wc = WindowConfig { opacity: -0.5, ..Default::default() };
    assert!((wc.clamped_opacity() - 0.0).abs() < f32::EPSILON);
    let wc = WindowConfig { opacity: 2.0, ..Default::default() };
    assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn window_padding_clamp() {
    // 200 を超える値は 200 にクランプされる
    let wc = WindowConfig { padding_x: 500, padding_y: 300, ..Default::default() };
    assert_eq!(wc.clamped_padding_x(), 200);
    assert_eq!(wc.clamped_padding_y(), 200);
    // 範囲内の値はそのまま
    let wc2 = WindowConfig { padding_x: 8, padding_y: 4, ..Default::default() };
    assert_eq!(wc2.clamped_padding_x(), 8);
    assert_eq!(wc2.clamped_padding_y(), 4);
}

#[test]
fn window_opacity_clamp_nan_inf() {
    let wc = WindowConfig { opacity: f32::NAN, ..Default::default() };
    assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
    let wc = WindowConfig { opacity: f32::INFINITY, ..Default::default() };
    assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
    let wc = WindowConfig { opacity: f32::NEG_INFINITY, ..Default::default() };
    assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn paste_config_default() {
    let pc = PasteConfig::default();
    assert!(pc.confirm_multiline);
}

#[test]
fn paste_config_deserialize() {
    let toml_str = "[paste]\nconfirm_multiline = false\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.paste.confirm_multiline);
}

#[test]
fn notification_config_default() {
    let nc = NotificationConfig::default();
    assert!(nc.enabled);
}

#[test]
fn notification_config_deserialize() {
    let toml_str = "[notification]\nenabled = false\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.notification.enabled);
}

#[test]
fn config_save_with_comments_is_parseable() {
    let config = Config::default();
    let path = std::path::PathBuf::from("tmp/test-config-comments.toml");
    std::fs::create_dir_all("tmp").expect("tmp dir");
    // create_new(true) を使うため、既存ファイルを先に削除する
    let _ = std::fs::remove_file(&path);
    config.save_with_comments(&path).expect("save failed");
    let loaded = Config::load(&path);
    assert!(
        (loaded.font.size - config.font.size).abs() < f32::EPSILON,
        "font.size mismatch after comment-save: {} vs {}",
        loaded.font.size,
        config.font.size
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("# SDIT"), "expected '# SDIT' comment header");
    assert!(content.contains("[font]"), "expected [font] section");
    let _ = std::fs::remove_file(&path);
}

// -----------------------------------------------------------------------
// CursorConfig テスト
// -----------------------------------------------------------------------

#[test]
fn cursor_config_default() {
    let cc = CursorConfig::default();
    assert_eq!(cc.style, CursorStyleConfig::Block);
    assert!(!cc.blinking);
    assert!(cc.color.is_none());
}

#[test]
fn cursor_config_deserialize_full() {
    let toml_str = "[cursor]\nstyle = \"bar\"\nblinking = true\ncolor = \"#ff6600\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.cursor.style, CursorStyleConfig::Bar);
    assert!(config.cursor.blinking);
    assert_eq!(config.cursor.color.as_deref(), Some("#ff6600"));
}

#[test]
fn cursor_config_deserialize_partial() {
    let toml_str = "[cursor]\nstyle = \"underline\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.cursor.style, CursorStyleConfig::Underline);
    assert!(!config.cursor.blinking); // default
    assert!(config.cursor.color.is_none()); // default
}

#[test]
fn cursor_config_deserialize_empty_uses_defaults() {
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.cursor.style, CursorStyleConfig::Block);
    assert!(!config.cursor.blinking);
    assert!(config.cursor.color.is_none());
}

#[test]
fn cursor_style_config_converts_to_cursor_style() {
    use crate::terminal::CursorStyle;
    assert_eq!(CursorStyle::from(CursorStyleConfig::Block), CursorStyle::Block);
    assert_eq!(CursorStyle::from(CursorStyleConfig::Underline), CursorStyle::Underline);
    assert_eq!(CursorStyle::from(CursorStyleConfig::Bar), CursorStyle::Bar);
}

// -----------------------------------------------------------------------
// ScrollbackConfig テスト
// -----------------------------------------------------------------------

#[test]
fn scrollback_config_default() {
    let sc = ScrollbackConfig::default();
    assert_eq!(sc.lines, 10_000);
}

#[test]
fn scrollback_config_deserialize() {
    let toml_str = "[scrollback]\nlines = 50000\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.scrollback.lines, 50_000);
}

#[test]
fn scrollback_clamped_lines_zero() {
    let sc = ScrollbackConfig { lines: 0 };
    assert_eq!(sc.clamped_lines(), 0);
}

#[test]
fn scrollback_clamped_lines_over_max() {
    let sc = ScrollbackConfig { lines: 2_000_000 };
    assert_eq!(sc.clamped_lines(), 1_000_000);
}

#[test]
fn scrollback_config_empty_uses_default() {
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.scrollback.lines, 10_000);
}

// -----------------------------------------------------------------------
// WindowConfig columns/rows テスト
// -----------------------------------------------------------------------

#[test]
fn window_config_columns_rows_default() {
    let wc = WindowConfig::default();
    assert_eq!(wc.columns, 80);
    assert_eq!(wc.rows, 24);
}

#[test]
fn window_config_columns_rows_deserialize() {
    let toml_str = "[window]\ncolumns = 120\nrows = 36\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.columns, 120);
    assert_eq!(config.window.rows, 36);
}

#[test]
fn window_config_columns_clamp() {
    let wc = WindowConfig { columns: 5, ..Default::default() };
    assert_eq!(wc.clamped_columns(), 10);
    let wc = WindowConfig { columns: 600, ..Default::default() };
    assert_eq!(wc.clamped_columns(), 500);
    let wc = WindowConfig { columns: 80, ..Default::default() };
    assert_eq!(wc.clamped_columns(), 80);
}

#[test]
fn window_config_rows_clamp() {
    let wc = WindowConfig { rows: 1, ..Default::default() };
    assert_eq!(wc.clamped_rows(), 2);
    let wc = WindowConfig { rows: 300, ..Default::default() };
    assert_eq!(wc.clamped_rows(), 200);
    let wc = WindowConfig { rows: 24, ..Default::default() };
    assert_eq!(wc.clamped_rows(), 24);
}

// -----------------------------------------------------------------------
// ScrollingConfig のテスト
// -----------------------------------------------------------------------

#[test]
fn scrolling_config_default() {
    let sc = ScrollingConfig::default();
    assert_eq!(sc.multiplier, 3);
    assert_eq!(sc.clamped_multiplier(), 3);
}

#[test]
fn scrolling_config_clamp_min() {
    let sc = ScrollingConfig { multiplier: 0, ..Default::default() };
    assert_eq!(sc.clamped_multiplier(), 1);
}

#[test]
fn scrolling_config_clamp_max() {
    let sc = ScrollingConfig { multiplier: 999, ..Default::default() };
    assert_eq!(sc.clamped_multiplier(), 100);
}

#[test]
fn scrolling_config_deserialize() {
    let toml_str = "[scrolling]\nmultiplier = 5\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.scrolling.multiplier, 5);
}

// -----------------------------------------------------------------------
// SelectionConfig のテスト
// -----------------------------------------------------------------------

#[test]
fn selection_config_default() {
    let sc = SelectionConfig::default();
    assert!(sc.word_chars.is_empty());
    assert!(!sc.save_to_clipboard);
}

#[test]
fn selection_config_word_chars_clamp() {
    // 256 文字以内はそのまま
    let s256: String = "a".repeat(256);
    let sc = SelectionConfig { word_chars: s256.clone(), ..Default::default() };
    assert_eq!(sc.clamped_word_chars().chars().count(), 256);

    // 257 文字は 256 文字にクランプ
    let s257: String = "b".repeat(257);
    let sc2 = SelectionConfig { word_chars: s257, ..Default::default() };
    assert_eq!(sc2.clamped_word_chars().chars().count(), 256);
}

#[test]
fn selection_config_deserialize() {
    let toml_str = "[selection]\nword_chars = \"-.\"\nsave_to_clipboard = true\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.selection.word_chars, "-.");
    assert!(config.selection.save_to_clipboard);
}

// -----------------------------------------------------------------------
// MouseConfig のテスト
// -----------------------------------------------------------------------

#[test]
fn mouse_config_default() {
    let mc = MouseConfig::default();
    assert!(!mc.hide_when_typing);
}

#[test]
fn mouse_config_deserialize() {
    let toml_str = "[mouse]\nhide_when_typing = true\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.mouse.hide_when_typing);
}

// -----------------------------------------------------------------------
// LinkConfig のテスト
// -----------------------------------------------------------------------

#[test]
fn link_config_deserialize() {
    let toml_str = r#"
[[links]]
regex = "JIRA-\\d+"
action = "open:https://jira.example.com/browse/$0"

[[links]]
regex = "GH-\\d+"
action = "open:https://github.com/issues/$0"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.links.len(), 2);
    assert_eq!(config.links[0].regex, "JIRA-\\d+");
    assert_eq!(config.links[0].action, "open:https://jira.example.com/browse/$0");
    assert_eq!(config.links[1].regex, "GH-\\d+");
}

#[test]
fn link_config_empty_by_default() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.links.is_empty());
}

#[test]
fn link_config_entry_limit() {
    // 33 エントリを設定しても clamped_links は 32 件しか返さない
    let entries: String = (0..33)
        .map(|i| format!("[[links]]\nregex = \"PAT-{i}\"\naction = \"open:https://example.com\"\n"))
        .collect();
    let config: Config = toml::from_str(&entries).unwrap();
    assert_eq!(config.links.len(), 33);
    assert_eq!(config.clamped_links().count(), 32);
}

#[test]
fn link_config_regex_length_limit() {
    // regex が 512 文字を超えるエントリは clamped_links に含まれない
    let long_regex = "a".repeat(513);
    let toml_str =
        format!("[[links]]\nregex = \"{long_regex}\"\naction = \"open:https://example.com\"\n");
    let config: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(config.clamped_links().count(), 0);
}

#[test]
fn link_config_action_length_limit() {
    // action が 1024 文字を超えるエントリは clamped_links に含まれない
    let long_action = format!("open:https://example.com/{}", "a".repeat(1000));
    let toml_str = format!("[[links]]\nregex = \"PAT\"\naction = \"{long_action}\"\n");
    let config: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(config.clamped_links().count(), 0);
}

// -----------------------------------------------------------------------
// Phase 16 新設定のテスト
// -----------------------------------------------------------------------

#[test]
fn startup_mode_default_is_windowed() {
    let wc = WindowConfig::default();
    assert_eq!(wc.startup_mode, StartupMode::Windowed);
}

#[test]
fn startup_mode_deserialize() {
    let cases: &[(&str, StartupMode)] = &[
        ("[window]\nstartup_mode = \"Windowed\"\n", StartupMode::Windowed),
        ("[window]\nstartup_mode = \"Maximized\"\n", StartupMode::Maximized),
        ("[window]\nstartup_mode = \"Fullscreen\"\n", StartupMode::Fullscreen),
    ];
    for (toml_str, expected) in cases {
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.window.startup_mode, *expected, "failed for: {toml_str}");
    }
}

#[test]
fn window_config_inherit_working_directory_default() {
    let wc = WindowConfig::default();
    assert!(wc.inherit_working_directory);
}

#[test]
fn window_config_inherit_working_directory_deserialize() {
    let toml_str = "[window]\ninherit_working_directory = false\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.window.inherit_working_directory);
}

#[test]
fn selection_trim_trailing_spaces_default() {
    let sc = SelectionConfig::default();
    assert!(sc.trim_trailing_spaces);
}

#[test]
fn selection_trim_trailing_spaces_deserialize() {
    let toml_str = "[selection]\ntrim_trailing_spaces = false\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.selection.trim_trailing_spaces);
}

#[test]
fn scrolling_scroll_to_bottom_default() {
    let sc = ScrollingConfig::default();
    assert!(sc.scroll_to_bottom_on_keystroke);
    assert!(!sc.scroll_to_bottom_on_output);
}

#[test]
fn scrolling_scroll_to_bottom_deserialize() {
    let toml_str =
        "[scrolling]\nscroll_to_bottom_on_keystroke = false\nscroll_to_bottom_on_output = true\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.scrolling.scroll_to_bottom_on_keystroke);
    assert!(config.scrolling.scroll_to_bottom_on_output);
}

// -----------------------------------------------------------------------
// ConfirmClose のテスト
// -----------------------------------------------------------------------

#[test]
fn confirm_close_default_is_process_running() {
    let wc = WindowConfig::default();
    assert_eq!(wc.confirm_close, ConfirmClose::ProcessRunning);
}

#[test]
fn confirm_close_deserialize_never() {
    let toml_str = "[window]\nconfirm_close = \"never\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.confirm_close, ConfirmClose::Never);
}

#[test]
fn confirm_close_deserialize_always() {
    let toml_str = "[window]\nconfirm_close = \"always\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.confirm_close, ConfirmClose::Always);
}

#[test]
fn confirm_close_deserialize_process_running() {
    let toml_str = "[window]\nconfirm_close = \"process_running\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.confirm_close, ConfirmClose::ProcessRunning);
}

#[test]
fn confirm_close_default_in_full_config() {
    let cfg = Config::default();
    assert_eq!(cfg.window.confirm_close, ConfirmClose::ProcessRunning);
}

// -----------------------------------------------------------------------
// Decorations のテスト
// -----------------------------------------------------------------------

#[test]
fn decorations_default_is_full() {
    let wc = WindowConfig::default();
    assert_eq!(wc.decorations, Decorations::Full);
}

#[test]
fn decorations_deserialize_none() {
    let toml_str = "[window]\ndecorations = \"none\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.decorations, Decorations::None);
}

#[test]
fn decorations_deserialize_full() {
    let toml_str = "[window]\ndecorations = \"full\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.decorations, Decorations::Full);
}

// -----------------------------------------------------------------------
// always_on_top のテスト
// -----------------------------------------------------------------------

#[test]
fn always_on_top_default_is_false() {
    let wc = WindowConfig::default();
    assert!(!wc.always_on_top);
}

#[test]
fn always_on_top_deserialize() {
    let toml_str = "[window]\nalways_on_top = true\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.window.always_on_top);
}

// -----------------------------------------------------------------------
// RightClickAction のテスト
// -----------------------------------------------------------------------

#[test]
fn right_click_action_default_is_context_menu() {
    let mc = MouseConfig::default();
    assert_eq!(mc.right_click_action, RightClickAction::ContextMenu);
}

#[test]
fn right_click_action_deserialize_paste() {
    let toml_str = "[mouse]\nright_click_action = \"paste\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.mouse.right_click_action, RightClickAction::Paste);
}

#[test]
fn right_click_action_deserialize_none() {
    let toml_str = "[mouse]\nright_click_action = \"none\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.mouse.right_click_action, RightClickAction::None);
}

#[test]
fn right_click_action_deserialize_context_menu() {
    let toml_str = "[mouse]\nright_click_action = \"context_menu\"\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.mouse.right_click_action, RightClickAction::ContextMenu);
}

// -----------------------------------------------------------------------
// restore_session のテスト
// -----------------------------------------------------------------------

#[test]
fn restore_session_default_is_true() {
    let window = WindowConfig::default();
    assert!(window.restore_session, "restore_session should default to true");
}

#[test]
fn restore_session_deserialize_false() {
    let toml_str = "[window]\nrestore_session = false\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.window.restore_session);
}

#[test]
fn restore_session_deserialize_true_explicit() {
    let toml_str = "[window]\nrestore_session = true\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.window.restore_session);
}

#[test]
fn restore_session_missing_field_defaults_to_true() {
    // restore_session フィールドなしの TOML を読んだらデフォルト true になる
    let toml_str = "[window]\nopacity = 1.0\n";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.window.restore_session, "missing field should default to true");
}

// -----------------------------------------------------------------------
// Phase 18.1: 背景画像設定テスト
// -----------------------------------------------------------------------

#[test]
fn background_image_default_is_none() {
    let window = WindowConfig::default();
    assert!(window.background_image.is_none());
    assert!((window.background_image_opacity - 0.3).abs() < f32::EPSILON);
    assert_eq!(window.background_image_fit, crate::config::BackgroundImageFit::Cover);
}

#[test]
fn background_image_deserialize() {
    let toml_str = r#"
[window]
background_image = "~/Pictures/bg.png"
background_image_opacity = 0.5
background_image_fit = "contain"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.background_image.as_deref(), Some("~/Pictures/bg.png"));
    assert!((config.window.background_image_opacity - 0.5).abs() < f32::EPSILON);
    assert_eq!(config.window.background_image_fit, crate::config::BackgroundImageFit::Contain);
}

#[test]
fn background_image_opacity_clamped() {
    let window = WindowConfig { background_image_opacity: 2.0, ..WindowConfig::default() };
    assert!((window.clamped_background_image_opacity() - 1.0).abs() < f32::EPSILON);
    let window = WindowConfig { background_image_opacity: -1.0, ..WindowConfig::default() };
    assert!((window.clamped_background_image_opacity() - 0.0).abs() < f32::EPSILON);
    let window = WindowConfig { background_image_opacity: f32::NAN, ..WindowConfig::default() };
    assert!((window.clamped_background_image_opacity() - 0.3).abs() < f32::EPSILON);
}

#[test]
fn background_image_fit_variants() {
    let toml_str = "[window]\nbackground_image_fit = \"fill\"";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.background_image_fit, crate::config::BackgroundImageFit::Fill);

    let toml_str = "[window]\nbackground_image_fit = \"cover\"";
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.background_image_fit, crate::config::BackgroundImageFit::Cover);
}

// -----------------------------------------------------------------------
// Phase 18.4: clipboard_codepoint_map テスト
// -----------------------------------------------------------------------

#[test]
fn clipboard_codepoint_map_default_is_empty() {
    let sel = SelectionConfig::default();
    assert!(sel.clipboard_codepoint_map.is_empty());
}

#[test]
fn clipboard_codepoint_map_deserialize() {
    let toml_str = r#"
[selection.clipboard_codepoint_map]
"U+2500-U+257F" = " "
"U+0041" = "A"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.selection.clipboard_codepoint_map.len(), 2);
}

#[test]
fn apply_codepoint_map_empty_map() {
    let sel = SelectionConfig::default();
    let text = "hello\u{2500}world";
    assert_eq!(sel.apply_codepoint_map(text), text);
}

#[test]
fn apply_codepoint_map_replaces_characters() {
    let mut sel = SelectionConfig::default();
    sel.clipboard_codepoint_map.insert("U+2500-U+257F".to_owned(), "-".to_owned());
    let text = "a\u{2500}b\u{2510}c";
    let result = sel.apply_codepoint_map(text);
    assert_eq!(result, "a-b-c");
}

#[test]
fn apply_codepoint_map_single_codepoint() {
    let mut sel = SelectionConfig::default();
    sel.clipboard_codepoint_map.insert("U+0041".to_owned(), "a".to_owned()); // 'A' → "a"
    let result = sel.apply_codepoint_map("ABC");
    assert_eq!(result, "aBC"); // A→a, B unchanged (0x42 not in range), C unchanged (0x43 not in range)
}

#[test]
fn apply_codepoint_map_passthrough_non_matching() {
    let mut sel = SelectionConfig::default();
    sel.clipboard_codepoint_map.insert("U+2500-U+257F".to_owned(), " ".to_owned());
    let text = "hello world";
    assert_eq!(sel.apply_codepoint_map(text), text);
}

#[test]
fn apply_codepoint_map_empty_replacement() {
    let mut sel = SelectionConfig::default();
    sel.clipboard_codepoint_map.insert("U+2500-U+257F".to_owned(), String::new());
    let text = "a\u{2500}b";
    let result = sel.apply_codepoint_map(text);
    assert_eq!(result, "ab");
}

// --- M-1: apply_codepoint_map DoS 防止テスト ---

#[test]
fn apply_codepoint_map_long_replacement_is_skipped() {
    // 257 文字の replacement は除外される（warn ログのみ、エントリ自体がスキップされる）
    let mut sel = SelectionConfig::default();
    let long_replacement = "x".repeat(257);
    sel.clipboard_codepoint_map.insert("U+0041".to_owned(), long_replacement); // 'A'
    let text = "ABC";
    // replacement が除外されるため、テキストは変換されずそのまま返る
    let result = sel.apply_codepoint_map(text);
    assert_eq!(result, "ABC", "長すぎる replacement はスキップされる");
}

#[test]
fn apply_codepoint_map_256_char_replacement_is_accepted() {
    // 256 文字の replacement は許容される
    let mut sel = SelectionConfig::default();
    let replacement = "y".repeat(256);
    sel.clipboard_codepoint_map.insert("U+0041".to_owned(), replacement.clone()); // 'A'
    let text = "A";
    let result = sel.apply_codepoint_map(text);
    assert_eq!(result, replacement, "256 文字の replacement は許容される");
}

#[test]
fn apply_codepoint_map_output_expansion_limit() {
    // 入力 1 文字 → replacement 256 文字 のマップで、入力 100 文字 = 出力 25600 文字
    // 入力 len = 100 bytes, max_output = 1000 bytes → 4 文字目付近で打ち切られる
    let mut sel = SelectionConfig::default();
    let replacement = "z".repeat(256); // 256 chars = 256 bytes (ASCII)
    sel.clipboard_codepoint_map.insert("U+0041".to_owned(), replacement); // 'A' → "zzz..."
    // 入力: "A" * 100, len = 100 bytes, max_output = 1000 bytes
    // 1 文字変換で 256 bytes 消費 → 4 文字目 (4*256=1024 > 1000) で打ち切り
    let text = "A".repeat(100);
    let result = sel.apply_codepoint_map(&text);
    // 出力は max_output (1000 bytes) 以内に収まる（最後の文字追加前にチェックするため若干超える場合あり）
    // 少なくとも全入力が変換されて 25600 文字にはならないことを確認
    assert!(result.len() < 25600, "出力膨張制限が機能していない: result.len() = {}", result.len());
}

// --- M-2: parse_clipboard_codepoint バリデーションテスト ---

#[test]
fn parse_clipboard_codepoint_empty_returns_none() {
    // 空文字列は None を返す
    assert!(super::parse_clipboard_codepoint("").is_none(), "空文字列は None を返す");
    assert!(super::parse_clipboard_codepoint("U+").is_none(), "U+ のみは None を返す");
}

#[test]
fn parse_clipboard_codepoint_too_long_returns_none() {
    // 9 文字以上の hex は None を返す
    assert!(
        super::parse_clipboard_codepoint("U+123456789").is_none(),
        "9 文字以上の hex は None を返す"
    );
    assert!(
        super::parse_clipboard_codepoint("123456789").is_none(),
        "9 文字以上の hex (prefix なし) は None を返す"
    );
}

#[test]
fn parse_clipboard_codepoint_non_hex_returns_none() {
    // 非 16 進数文字を含む場合は None を返す
    assert!(super::parse_clipboard_codepoint("U+00GG").is_none(), "非 16 進数文字は None を返す");
    assert!(
        super::parse_clipboard_codepoint("ZZZZ").is_none(),
        "非 16 進数文字 (prefix なし) は None を返す"
    );
}

#[test]
fn parse_clipboard_codepoint_valid_values() {
    // 正常なコードポイントは Some を返す
    assert_eq!(super::parse_clipboard_codepoint("U+0041"), Some(0x41)); // 'A'
    assert_eq!(super::parse_clipboard_codepoint("U+10FFFF"), Some(0x10FFFF));
    assert_eq!(super::parse_clipboard_codepoint("0041"), Some(0x41));
    assert_eq!(super::parse_clipboard_codepoint("u+0041"), Some(0x41)); // 小文字プレフィックス
}

#[test]
fn parse_clipboard_codepoint_out_of_range_returns_none() {
    // Unicode 範囲外 (> 0x10FFFF) は None を返す
    assert!(super::parse_clipboard_codepoint("U+110000").is_none(), "0x110000 は Unicode 範囲外");
}

// ---------------------------------------------------------------------------
// QuickTerminalConfig のテスト
// ---------------------------------------------------------------------------

#[test]
fn quick_terminal_default_values() {
    let cfg = QuickTerminalConfig::default();
    assert!(!cfg.enabled, "enabled のデフォルトは false");
    assert_eq!(cfg.position, QuickTerminalPosition::Top, "position のデフォルトは Top");
    assert!((cfg.size - 0.4).abs() < f32::EPSILON, "size のデフォルトは 0.4");
    assert_eq!(cfg.hotkey, "ctrl+`", "hotkey のデフォルトは ctrl+`");
    assert!(
        (cfg.animation_duration - 0.2).abs() < f32::EPSILON,
        "animation_duration のデフォルトは 0.2"
    );
}

#[test]
fn quick_terminal_deserialize() {
    let toml_str = r#"
[quick_terminal]
enabled = true
position = "bottom"
size = 0.5
hotkey = "ctrl+shift+t"
animation_duration = 0.3
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert!(cfg.quick_terminal.enabled);
    assert_eq!(cfg.quick_terminal.position, QuickTerminalPosition::Bottom);
    assert!((cfg.quick_terminal.size - 0.5).abs() < f32::EPSILON);
    assert_eq!(cfg.quick_terminal.hotkey, "ctrl+shift+t");
    assert!((cfg.quick_terminal.animation_duration - 0.3).abs() < f32::EPSILON);
}

#[test]
fn quick_terminal_size_clamped() {
    let cfg = QuickTerminalConfig { size: 0.05, ..Default::default() };
    assert!((cfg.clamped_size() - 0.1).abs() < f32::EPSILON, "0.05 は 0.1 にクランプ");

    let cfg = QuickTerminalConfig { size: 1.5, ..Default::default() };
    assert!((cfg.clamped_size() - 1.0).abs() < f32::EPSILON, "1.5 は 1.0 にクランプ");

    let cfg = QuickTerminalConfig { size: f32::NAN, ..Default::default() };
    assert!((cfg.clamped_size() - 0.4).abs() < f32::EPSILON, "NaN はデフォルト 0.4 を返す");
}

#[test]
fn quick_terminal_animation_duration_clamped() {
    let cfg = QuickTerminalConfig { animation_duration: -0.1, ..Default::default() };
    assert!(
        (cfg.clamped_animation_duration() - 0.0).abs() < f32::EPSILON,
        "-0.1 は 0.0 にクランプ"
    );

    let cfg = QuickTerminalConfig { animation_duration: 3.0, ..Default::default() };
    assert!((cfg.clamped_animation_duration() - 2.0).abs() < f32::EPSILON, "3.0 は 2.0 にクランプ");

    let cfg = QuickTerminalConfig { animation_duration: f32::INFINITY, ..Default::default() };
    assert!(
        (cfg.clamped_animation_duration() - 0.2).abs() < f32::EPSILON,
        "Inf はデフォルト 0.2 を返す"
    );
}

#[test]
fn quick_terminal_position_variants() {
    let positions = [
        ("top", QuickTerminalPosition::Top),
        ("bottom", QuickTerminalPosition::Bottom),
        ("left", QuickTerminalPosition::Left),
        ("right", QuickTerminalPosition::Right),
    ];
    for (s, expected) in positions {
        let toml_str = format!("[quick_terminal]\nposition = \"{s}\"\n");
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg.quick_terminal.position, expected, "position = {s}");
    }
}

#[test]
fn quick_terminal_default_in_full_config() {
    let cfg = Config::default();
    assert!(!cfg.quick_terminal.enabled);
    assert_eq!(cfg.quick_terminal.position, QuickTerminalPosition::Top);
    assert!((cfg.quick_terminal.size - 0.4).abs() < f32::EPSILON);
}

// -----------------------------------------------------------------------
// Phase 19.1: env + working_directory のテスト
// -----------------------------------------------------------------------

#[test]
fn env_default_is_empty() {
    let cfg = Config::default();
    assert!(cfg.env.is_empty(), "env のデフォルトは空 HashMap");
}

#[test]
fn env_deserialize() {
    let toml_str = r#"
[env]
TERM_PROGRAM = "sdit"
COLORTERM = "truecolor"
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.env.get("TERM_PROGRAM").map(String::as_str), Some("sdit"));
    assert_eq!(cfg.env.get("COLORTERM").map(String::as_str), Some("truecolor"));
}

#[test]
fn working_directory_default_is_none() {
    let cfg = Config::default();
    assert!(cfg.window.working_directory.is_none());
}

#[test]
fn working_directory_deserialize() {
    let toml_str = "[window]\nworking_directory = \"~/Projects\"\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.window.working_directory.as_deref(), Some("~/Projects"));
}

// -----------------------------------------------------------------------
// Phase 19.2: 検索色 + padding_color のテスト
// -----------------------------------------------------------------------

#[test]
fn search_colors_default_is_none() {
    let cfg = Config::default();
    assert!(cfg.colors.search_foreground.is_none());
    assert!(cfg.colors.search_background.is_none());
}

#[test]
fn search_colors_deserialize() {
    let toml_str = "[colors]\nsearch_foreground = \"#ffffff\"\nsearch_background = \"#ff8800\"\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.colors.search_foreground.as_deref(), Some("#ffffff"));
    assert_eq!(cfg.colors.search_background.as_deref(), Some("#ff8800"));
}

#[test]
fn padding_color_default_is_background() {
    let cfg = Config::default();
    assert_eq!(cfg.window.padding_color, PaddingColor::Background);
}

#[test]
fn padding_color_deserialize() {
    let toml_str = "[window]\npadding_color = \"background\"\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.window.padding_color, PaddingColor::Background);
}

// -----------------------------------------------------------------------
// Phase 19.4: click_repeat_interval + grapheme_width_method のテスト
// -----------------------------------------------------------------------

#[test]
fn click_repeat_interval_default_is_300() {
    let cfg = Config::default();
    assert_eq!(cfg.mouse.click_repeat_interval, 300);
}

#[test]
fn click_repeat_interval_deserialize() {
    let toml_str = "[mouse]\nclick_repeat_interval = 500\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.mouse.click_repeat_interval, 500);
}

#[test]
fn click_repeat_interval_clamped() {
    let mc = MouseConfig { click_repeat_interval: 10, ..Default::default() };
    assert_eq!(mc.clamped_click_repeat_interval(), 50, "下限は 50ms");

    let mc = MouseConfig { click_repeat_interval: 5000, ..Default::default() };
    assert_eq!(mc.clamped_click_repeat_interval(), 2000, "上限は 2000ms");

    let mc = MouseConfig { click_repeat_interval: 300, ..Default::default() };
    assert_eq!(mc.clamped_click_repeat_interval(), 300, "範囲内はそのまま");
}

#[test]
fn grapheme_width_method_default_is_unicode() {
    let cfg = Config::default();
    assert_eq!(cfg.terminal.grapheme_width_method, GraphemeWidthMethod::Unicode);
}

#[test]
fn grapheme_width_method_deserialize() {
    let cases =
        [("unicode", GraphemeWidthMethod::Unicode), ("legacy", GraphemeWidthMethod::Legacy)];
    for (s, expected) in cases {
        let toml_str = format!("[terminal]\ngrapheme_width_method = \"{s}\"\n");
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg.terminal.grapheme_width_method, expected, "failed for {s}");
    }
}

// -----------------------------------------------------------------------
// Phase 19.5: window_subtitle のテスト
// -----------------------------------------------------------------------

#[test]
fn window_subtitle_default_is_none() {
    let cfg = Config::default();
    assert_eq!(cfg.window.subtitle, WindowSubtitle::None);
}

#[test]
fn window_subtitle_deserialize() {
    let cases = [
        ("none", WindowSubtitle::None),
        ("working-directory", WindowSubtitle::WorkingDirectory),
        ("session-name", WindowSubtitle::SessionName),
    ];
    for (s, expected) in cases {
        let toml_str = format!("[window]\nsubtitle = \"{s}\"\n");
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg.window.subtitle, expected, "failed for {s}");
    }
}

#[test]
fn window_position_config_default() {
    let cfg = WindowConfig::default();
    assert!(cfg.position_x.is_none(), "position_x のデフォルトは None");
    assert!(cfg.position_y.is_none(), "position_y のデフォルトは None");
}

#[test]
fn window_position_config_deserialize() {
    let toml_str = "[window]\nposition_x = 100\nposition_y = 200\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.window.position_x, Some(100));
    assert_eq!(cfg.window.position_y, Some(200));
}

#[test]
fn focus_follows_mouse_config_default() {
    let cfg = MouseConfig::default();
    assert!(!cfg.focus_follows_mouse, "focus_follows_mouse のデフォルトは false");
}

#[test]
fn focus_follows_mouse_config_deserialize() {
    let toml_str = "[mouse]\nfocus_follows_mouse = true\n";
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert!(cfg.mouse.focus_follows_mouse);
}

#[test]
fn window_position_clamp() {
    // 両方 None のとき: None を返す
    let mut cfg = WindowConfig::default();
    assert_eq!(cfg.clamped_position(), None, "position_x/y が両方 None なら None");

    // 片方のみ Some のとき: None を返す
    cfg.position_x = Some(100);
    cfg.position_y = None;
    assert_eq!(cfg.clamped_position(), None, "position_x のみ Some でも None");

    cfg.position_x = None;
    cfg.position_y = Some(200);
    assert_eq!(cfg.clamped_position(), None, "position_y のみ Some でも None");

    // 通常値はそのまま返す
    cfg.position_x = Some(100);
    cfg.position_y = Some(200);
    assert_eq!(cfg.clamped_position(), Some((100, 200)), "通常値はそのまま");

    // 上限を超える値はクランプされる
    cfg.position_x = Some(999_999);
    cfg.position_y = Some(999_999);
    assert_eq!(cfg.clamped_position(), Some((32000, 32000)), "上限 999999 → 32000 にクランプ");

    // 負方向の極端な値もクランプされる
    cfg.position_x = Some(-10_000_000);
    cfg.position_y = Some(-10_000_000);
    assert_eq!(
        cfg.clamped_position(),
        Some((-16000, -16000)),
        "下限 -10000000 → -16000 にクランプ"
    );

    // 境界値はそのまま通る
    cfg.position_x = Some(-16000);
    cfg.position_y = Some(32000);
    assert_eq!(cfg.clamped_position(), Some((-16000, 32000)), "境界値はそのまま");
}
