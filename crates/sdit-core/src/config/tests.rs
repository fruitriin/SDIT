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
