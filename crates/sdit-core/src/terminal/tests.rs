use super::*;
use crate::index::{Column, Line, Point};

fn make_proc_term(lines: usize, cols: usize) -> (Processor, Terminal) {
    (Processor::new(), Terminal::new(lines, cols, 100))
}

// 1. print テスト: "Hello" を送り込んでセルを確認する
#[test]
fn print_hello() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"Hello");
    assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'H');
    assert_eq!(term.grid()[Point::new(Line(0), Column(1))].c, 'e');
    assert_eq!(term.grid()[Point::new(Line(0), Column(4))].c, 'o');
    // cursor is at column 5
    assert_eq!(term.grid().cursor.point.column, Column(5));
}

// 2. 改行テスト: LF+CR でカーソルが次の行頭に移動する
#[test]
fn linefeed_and_cr() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"A\r\nB");
    // 'A' at (0,0), 'B' at (1,0)
    assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'A');
    assert_eq!(term.grid()[Point::new(Line(1), Column(0))].c, 'B');
    assert_eq!(term.grid().cursor.point.line, Line(1));
    assert_eq!(term.grid().cursor.point.column, Column(1));
}

// 3. カーソル移動テスト: CUP (ESC[row;colH)
#[test]
fn cursor_position() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // ESC [ 5 ; 10 H  →  line=4, col=9 (1-based → 0-based)
    proc.advance(&mut term, b"\x1b[5;10H");
    assert_eq!(term.grid().cursor.point.line, Line(4));
    assert_eq!(term.grid().cursor.point.column, Column(9));
}

// 4. SGR テスト: SGR 1 (BOLD) → print → セルの flags に BOLD が含まれる
#[test]
fn sgr_bold() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[1mX");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, 'X');
    assert!(cell.flags.contains(CellFlags::BOLD));
}

// 5. 画面消去テスト: ED 2 (ESC[2J) で全画面消去
#[test]
fn erase_display_all() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"Hello");
    proc.advance(&mut term, b"\x1b[2J");
    // Entire screen should be blank.
    for line in 0..24_i32 {
        for col in 0..80 {
            let c = term.grid()[Point::new(Line(line), Column(col))].c;
            assert_eq!(c, ' ', "expected space at ({line},{col})");
        }
    }
}

// 6. スクロールテスト: 画面下端で LF → scroll_up が発生
#[test]
fn scroll_at_bottom() {
    let (mut proc, mut term) = make_proc_term(5, 10);
    // Fill all 5 lines, then add an extra LF.
    proc.advance(&mut term, b"A\r\nB\r\nC\r\nD\r\nE");
    let history_before = term.grid().history_size();
    proc.advance(&mut term, b"\r\n");
    // After LF from line 4 (0-indexed bottom), scroll_up should have fired.
    assert!(
        term.grid().history_size() > history_before,
        "expected scroll_up to push a line into history"
    );
}

// 7. alt screen テスト: 切替→復帰でプライマリ画面が復元される
#[test]
fn alt_screen_roundtrip() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // Write to primary.
    proc.advance(&mut term, b"Primary");
    let primary_char = term.grid()[Point::new(Line(0), Column(0))].c;
    assert_eq!(primary_char, 'P');

    // Switch to alt screen.
    proc.advance(&mut term, b"\x1b[?1049h");
    assert!(term.mode().contains(TermMode::ALT_SCREEN));
    // Alt screen should be blank.
    assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, ' ');

    // Write something on alt screen.
    proc.advance(&mut term, b"Alt");

    // Switch back.
    proc.advance(&mut term, b"\x1b[?1049l");
    assert!(!term.mode().contains(TermMode::ALT_SCREEN));
    // Primary content should be restored.
    assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'P');
}

// 8. Processor テスト: バイト列を渡して Grid 状態を確認
#[test]
fn processor_advance_complex() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // Write "AB", move cursor to (0,0), overwrite with "CD".
    proc.advance(&mut term, b"AB\x1b[1;1HCD");
    assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'C');
    assert_eq!(term.grid()[Point::new(Line(0), Column(1))].c, 'D');
}

// Additional: SGR reset
#[test]
fn sgr_reset() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[1;3m"); // BOLD + ITALIC
    assert!(term.grid().cursor.template.flags.contains(CellFlags::BOLD));
    proc.advance(&mut term, b"\x1b[0m"); // reset
    assert!(!term.grid().cursor.template.flags.contains(CellFlags::BOLD));
    assert!(!term.grid().cursor.template.flags.contains(CellFlags::ITALIC));
}

// Additional: cursor up / down / left / right
#[test]
fn cursor_movements() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[10;20H"); // line=9, col=19
    proc.advance(&mut term, b"\x1b[2A"); // up 2 → line=7
    assert_eq!(term.grid().cursor.point.line, Line(7));
    proc.advance(&mut term, b"\x1b[3B"); // down 3 → line=10
    assert_eq!(term.grid().cursor.point.line, Line(10));
    proc.advance(&mut term, b"\x1b[5C"); // right 5 → col=24
    assert_eq!(term.grid().cursor.point.column, Column(24));
    proc.advance(&mut term, b"\x1b[10D"); // left 10 → col=14
    assert_eq!(term.grid().cursor.point.column, Column(14));
}

// Additional: erase line
#[test]
fn erase_line() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"Hello World");
    proc.advance(&mut term, b"\x1b[2K"); // erase whole line
    for col in 0..80 {
        assert_eq!(term.grid()[Point::new(Line(0), Column(col))].c, ' ', "col {col} not blank");
    }
}

// Additional: OSC title length limit (defense-in-depth)
#[test]
fn osc_title_capped() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // Build an OSC 0 sequence with a long title.
    // Note: vte 0.13's internal buffer truncates OSC params at ~1024 bytes,
    // so the title that reaches our handler is already capped by the parser.
    // Our MAX_TITLE_BYTES (4096) is defense-in-depth for future vte changes.
    let mut seq = Vec::new();
    seq.extend_from_slice(b"\x1b]0;");
    seq.extend(std::iter::repeat_n(b'A', 5000));
    seq.push(0x07); // BEL
    proc.advance(&mut term, &seq);
    let title = term.title().expect("title should be set");
    // The title must be bounded (vte caps at ~1024, our limit at 4096).
    assert!(title.len() <= 4096);
    assert!(!title.is_empty());
}

// Test our OSC handler directly to verify the 4096 cap logic.
#[test]
fn osc_title_direct_cap_at_4096() {
    let mut term = Terminal::new(24, 80, 100);
    // Simulate OSC dispatch with a payload exceeding 4096 bytes,
    // bypassing vte's parser buffer limit.
    let long_title: Vec<u8> = std::iter::repeat_n(b'B', 5000).collect();
    term.osc_dispatch(&[b"0", &long_title], false);
    let title = term.title().expect("title should be set");
    assert_eq!(title.len(), 4096);
}

// Additional: scroll region (DECSTBM)
#[test]
fn decstbm_sets_scroll_region() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[5;20r");
    assert_eq!(term.scroll_region, 4..20);
}

// Additional: IL/DL
#[test]
fn insert_and_delete_lines() {
    let (mut proc, mut term) = make_proc_term(10, 20);
    // Write to line 0.
    proc.advance(&mut term, b"Line0");
    proc.advance(&mut term, b"\r\nLine1");
    // Cursor is now at (1, 5). Insert 1 line at current line.
    proc.advance(&mut term, b"\x1b[1L");
    // "Line0" should still be at line 0.
    assert_eq!(term.grid()[Point::new(Line(0), Column(0))].c, 'L');
    // Line 1 should now be blank (inserted line).
    assert_eq!(term.grid()[Point::new(Line(1), Column(0))].c, ' ');
    // Former line 1 ("Line1") should be at line 2.
    assert_eq!(term.grid()[Point::new(Line(2), Column(0))].c, 'L');

    // Delete the inserted blank line (cursor is still at line 1).
    proc.advance(&mut term, b"\x1b[1;1H\x1b[1;1H"); // reposition for clarity
    proc.advance(&mut term, b"\x1b[2;1H"); // move to line 2 (1-based = line index 1)
    proc.advance(&mut term, b"\x1b[1M");
    // Line 1 should now be "Line1" again.
    assert_eq!(term.grid()[Point::new(Line(1), Column(0))].c, 'L');
}

// CJK: 全角文字が2セル幅で配置される
#[test]
fn cjk_wide_char() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // 日本語の "あ" は全角（幅2）
    proc.advance(&mut term, "あ".as_bytes());
    let cell0 = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell0.c, 'あ');
    assert!(cell0.flags.contains(CellFlags::WIDE_CHAR));

    let cell1 = &term.grid()[Point::new(Line(0), Column(1))];
    assert!(cell1.flags.contains(CellFlags::WIDE_CHAR_SPACER));

    // カーソルは列2に進んでいる
    assert_eq!(term.grid().cursor.point.column, Column(2));
}

// CJK: 行末で全角文字がはみ出す場合のラップ
#[test]
fn cjk_wrap_at_line_end() {
    let (mut proc, mut term) = make_proc_term(24, 10);
    // 列0〜8に9文字書いて、列9に全角文字を書く（はみ出すのでラップ）
    proc.advance(&mut term, b"123456789");
    proc.advance(&mut term, "あ".as_bytes());

    // "あ" は次の行の先頭に配置される
    let cell = &term.grid()[Point::new(Line(1), Column(0))];
    assert_eq!(cell.c, 'あ');
    assert!(cell.flags.contains(CellFlags::WIDE_CHAR));
}

// CJK: unicode_width による幅判定
#[test]
fn cjk_unicode_width() {
    use unicode_width::UnicodeWidthChar;
    assert_eq!(UnicodeWidthChar::width('あ'), Some(2));
    assert_eq!(UnicodeWidthChar::width('漢'), Some(2));
    assert_eq!(UnicodeWidthChar::width('A'), Some(1));
    assert_eq!(UnicodeWidthChar::width('ｱ'), Some(1)); // 半角カタカナ
}

// DA1 応答テスト
#[test]
fn da1_response() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[c");
    let response = term.drain_pending_writes().unwrap();
    assert_eq!(response, b"\x1b[?62;4c");
}

// DA2 応答テスト
#[test]
fn da2_response() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[>c");
    let response = term.drain_pending_writes().unwrap();
    assert_eq!(response, b"\x1b[>0;0;0c");
}

// DSR 応答テスト
#[test]
fn dsr_status_ok() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[5n");
    let response = term.drain_pending_writes().unwrap();
    assert_eq!(response, b"\x1b[0n");
}

// CPR 応答テスト
#[test]
fn cpr_cursor_position_report() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[5;10H"); // move to (5,10) 1-based
    proc.advance(&mut term, b"\x1b[6n");
    let response = term.drain_pending_writes().unwrap();
    assert_eq!(response, b"\x1b[5;10R");
}

// DECSCUSR テスト
#[test]
fn decscusr_cursor_style() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // Bar (blinking)
    proc.advance(&mut term, b"\x1b[5 q");
    assert_eq!(term.cursor_style(), CursorStyle::Bar);
    assert!(term.cursor_blinking());
    // Underline (steady)
    proc.advance(&mut term, b"\x1b[4 q");
    assert_eq!(term.cursor_style(), CursorStyle::Underline);
    assert!(!term.cursor_blinking());
    // Block (blinking)
    proc.advance(&mut term, b"\x1b[1 q");
    assert_eq!(term.cursor_style(), CursorStyle::Block);
    assert!(term.cursor_blinking());
}

// マウスモード: CSI ? 1000 h で MOUSE_REPORT_CLICK がセットされる
#[test]
fn mouse_mode_click_set() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[?1000h");
    assert!(term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(term.mouse_mode_active());
}

// マウスモード: CSI ? 1006 h で SGR_MOUSE がセットされる
#[test]
fn mouse_mode_sgr_set() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[?1006h");
    assert!(term.mode().contains(TermMode::SGR_MOUSE));
}

// マウスモード: CSI ? 1000 l でリセットされる
#[test]
fn mouse_mode_click_reset() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[?1000h");
    assert!(term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    proc.advance(&mut term, b"\x1b[?1000l");
    assert!(!term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(!term.mouse_mode_active());
}

// マウスモード: X10 (?9) / drag (?1002) / motion (?1003) も設定できる
#[test]
fn mouse_mode_variants() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b[?9h");
    assert!(term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    proc.advance(&mut term, b"\x1b[?9l");
    proc.advance(&mut term, b"\x1b[?1002h");
    assert!(term.mode().contains(TermMode::MOUSE_REPORT_DRAG));
    assert!(term.mouse_mode_active());
    proc.advance(&mut term, b"\x1b[?1003h");
    assert!(term.mode().contains(TermMode::MOUSE_REPORT_MOTION));
    proc.advance(&mut term, b"\x1b[?1005h");
    assert!(term.mode().contains(TermMode::UTF8_MOUSE));
}

// OSC 52: クリップボード書き込みテスト
#[test]
fn osc52_clipboard_write() {
    let mut term = Terminal::new(24, 80, 0);
    // "Hello" の Base64 は "SGVsbG8="
    term.osc_dispatch(&[b"52", b"c", b"SGVsbG8="], false);
    assert_eq!(term.take_clipboard_write(), Some("Hello".to_string()));
    // 2回目は None（take セマンティクス）
    assert_eq!(term.take_clipboard_write(), None);
}

#[test]
fn osc52_clipboard_read_request_ignored() {
    let mut term = Terminal::new(24, 80, 0);
    // 読み取り要求 "?" は無視する
    term.osc_dispatch(&[b"52", b"c", b"?"], false);
    assert_eq!(term.take_clipboard_write(), None);
}

#[test]
fn osc52_invalid_base64_ignored() {
    let mut term = Terminal::new(24, 80, 0);
    term.osc_dispatch(&[b"52", b"c", b"not!valid!base64!!!"], false);
    // 不正な文字は None
    assert_eq!(term.take_clipboard_write(), None);
}

#[test]
fn decode_base64_basic() {
    // 内部ヘルパーを osc_dispatch 経由で間接テスト
    let mut term = Terminal::new(24, 80, 0);
    // "test" → "dGVzdA=="
    term.osc_dispatch(&[b"52", b"c", b"dGVzdA=="], false);
    assert_eq!(term.take_clipboard_write(), Some("test".to_string()));
}

// OSC 8: URL のセット・クリアテスト
#[test]
fn osc8_set_and_clear_hyperlink() {
    let mut term = Terminal::new(24, 80, 0);
    // URL をセット
    term.osc_dispatch(&[b"8", b"", b"https://example.com"], false);
    assert_eq!(term.current_hyperlink.as_deref(), Some("https://example.com"));
    // URL をクリア
    term.osc_dispatch(&[b"8", b"", b""], false);
    assert!(term.current_hyperlink.is_none());
}

#[test]
fn osc8_clear_with_two_params() {
    let mut term = Terminal::new(24, 80, 0);
    term.osc_dispatch(&[b"8", b"", b"https://example.com"], false);
    // params が2要素の場合もクリア
    term.osc_dispatch(&[b"8", b""], false);
    assert!(term.current_hyperlink.is_none());
}

// OSC 8: URL 長さ上限テスト
#[test]
fn osc8_url_length_limit() {
    let mut term = Terminal::new(24, 80, 0);
    // 2049 バイトの URL は拒否される
    let long_url: Vec<u8> = {
        let mut v = b"https://".to_vec();
        v.extend(std::iter::repeat_n(b'x', 2049 - 8));
        v
    };
    term.osc_dispatch(&[b"8", b"", &long_url], false);
    assert!(term.current_hyperlink.is_none());
}

// OSC 8: 不正 URL のフィルタリングテスト
#[test]
fn osc8_reject_non_http_url() {
    let mut term = Terminal::new(24, 80, 0);
    // file:// は拒否される
    term.osc_dispatch(&[b"8", b"", b"file:///etc/passwd"], false);
    assert!(term.current_hyperlink.is_none());
    // mailto: も拒否される
    term.osc_dispatch(&[b"8", b"", b"mailto:foo@example.com"], false);
    assert!(term.current_hyperlink.is_none());
    // http:// は受け付ける
    term.osc_dispatch(&[b"8", b"", b"http://example.com"], false);
    assert_eq!(term.current_hyperlink.as_deref(), Some("http://example.com"));
}

// OSC 8: セルへの hyperlink 統合テスト
#[test]
fn osc8_hyperlink_written_to_cells() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // OSC 8 でリンク開始してから文字を書く
    proc.advance(&mut term, b"\x1b]8;;https://example.com\x07");
    proc.advance(&mut term, b"Hello");
    proc.advance(&mut term, b"\x1b]8;;\x07"); // リンク終了
    proc.advance(&mut term, b"World");

    // "Hello" のセルには hyperlink が入っている
    for col in 0..5 {
        let cell = &term.grid()[Point::new(Line(0), Column(col))];
        assert_eq!(cell.hyperlink.as_deref(), Some("https://example.com"), "col {col}");
    }
    // "World" のセルには hyperlink がない
    for col in 5..10 {
        let cell = &term.grid()[Point::new(Line(0), Column(col))];
        assert!(cell.hyperlink.is_none(), "col {col} should have no hyperlink");
    }
}

// pending_writes サイズ制限テスト
#[test]
fn pending_writes_respects_max_limit() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // DA1 応答は 11 バイト ("\x1b[?62;4c")。MAX_PENDING_WRITES / 11 回以上送れば上限に達する。
    let repetitions = MAX_PENDING_WRITES / 11 + 100;
    for _ in 0..repetitions {
        proc.advance(&mut term, b"\x1b[c");
    }
    // バッファが MAX_PENDING_WRITES を超えていないこと。
    assert!(term.pending_writes.len() <= MAX_PENDING_WRITES);
    // 少なくとも何かは書き込まれていること。
    assert!(!term.pending_writes.is_empty());
}

// ---------------------------------------------------------------------------
// KittyFlagStack テスト
// ---------------------------------------------------------------------------

#[test]
fn kitty_flag_stack_default() {
    let stack = KittyFlagStack::default();
    assert_eq!(stack.current(), KittyKeyboardFlags::NONE);
}

#[test]
fn kitty_flag_stack_push_pop() {
    let mut stack = KittyFlagStack::default();
    stack.push(KittyKeyboardFlags::from_raw(1)); // disambiguate
    assert_eq!(stack.current().raw(), 1);
    stack.push(KittyKeyboardFlags::from_raw(3)); // disambiguate + report_events
    assert_eq!(stack.current().raw(), 3);
    stack.pop(1);
    assert_eq!(stack.current().raw(), 1);
    stack.pop(1);
    assert_eq!(stack.current(), KittyKeyboardFlags::NONE);
}

#[test]
fn kitty_flag_stack_pop_underflow() {
    let mut stack = KittyFlagStack::default();
    stack.push(KittyKeyboardFlags::from_raw(1));
    stack.pop(100); // 大量ポップ
    // 最低1エントリ（初期エントリ）が残る
    assert_eq!(stack.current(), KittyKeyboardFlags::NONE);
}

#[test]
fn kitty_flag_stack_overflow() {
    let mut stack = KittyFlagStack::default();
    // デフォルトで len=1 なので、あと7エントリまでプッシュ可能
    for i in 1..=7u8 {
        stack.push(KittyKeyboardFlags::from_raw(i & 0x1f));
    }
    // スタックが満杯（8エントリ）の状態で追加プッシュは無視される
    stack.push(KittyKeyboardFlags::from_raw(0x1f));
    stack.push(KittyKeyboardFlags::from_raw(0x1f));
    // 最後に正常プッシュしたフラグ（7 & 0x1f = 7）が残っている
    assert_eq!(stack.current().raw(), 7);
}

#[test]
fn kitty_flags_has() {
    let flags = KittyKeyboardFlags::from_raw(
        KittyKeyboardFlags::DISAMBIGUATE | KittyKeyboardFlags::REPORT_EVENTS,
    );
    assert!(flags.has(KittyKeyboardFlags::DISAMBIGUATE));
    assert!(flags.has(KittyKeyboardFlags::REPORT_EVENTS));
    assert!(!flags.has(KittyKeyboardFlags::REPORT_ALTERNATES));
    assert!(flags.is_active());
}

#[test]
fn kitty_flags_mask() {
    // 5ビット以上は切り捨てられる
    let flags = KittyKeyboardFlags::from_raw(0xff);
    assert_eq!(flags.raw(), 0x1f);
}

// Kitty CSI push/pop/query のシーケンステスト
#[test]
fn kitty_csi_push_pop_via_sequence() {
    let (mut proc, mut term) = make_proc_term(24, 80);

    // CSI > 1 u — push DISAMBIGUATE
    proc.advance(&mut term, b"\x1b[>1u");
    assert_eq!(term.kitty_flags.current().raw(), 1);

    // CSI > 3 u — push (DISAMBIGUATE | REPORT_EVENTS)
    proc.advance(&mut term, b"\x1b[>3u");
    assert_eq!(term.kitty_flags.current().raw(), 3);

    // CSI < 1 u — pop 1
    proc.advance(&mut term, b"\x1b[<1u");
    assert_eq!(term.kitty_flags.current().raw(), 1);

    // CSI < 1 u — pop 1 (initial entry)
    proc.advance(&mut term, b"\x1b[<1u");
    assert_eq!(term.kitty_flags.current(), KittyKeyboardFlags::NONE);
}

#[test]
fn kitty_csi_push_invalid_flags_clamped() {
    let (mut proc, mut term) = make_proc_term(24, 80);

    // CSI > 0xff u — flags_raw=255 (>0x1f) はクランプされて 0x1f になる
    proc.advance(&mut term, b"\x1b[>255u");
    assert_eq!(term.kitty_flags.current().raw(), 0x1f);
}

#[test]
fn kitty_csi_pop_large_n_clamped() {
    let (mut proc, mut term) = make_proc_term(24, 80);

    // スタックに2エントリ積む
    proc.advance(&mut term, b"\x1b[>1u");
    proc.advance(&mut term, b"\x1b[>3u");

    // CSI < 65535 u — n が上限8にクランプされるため安全にポップ
    proc.advance(&mut term, b"\x1b[<65535u");
    // 最低1エントリ（初期エントリ）が残る
    assert_eq!(term.kitty_flags.current(), KittyKeyboardFlags::NONE);
}

#[test]
fn kitty_csi_query_responds() {
    let (mut proc, mut term) = make_proc_term(24, 80);

    // Push DISAMBIGUATE
    proc.advance(&mut term, b"\x1b[>1u");

    // CSI ? u — query
    proc.advance(&mut term, b"\x1b[?u");
    let response = term.drain_pending_writes().unwrap();
    assert_eq!(response, b"\x1b[?1u");
}

// OSC 9/99 デスクトップ通知テスト
#[test]
fn osc_9_notification() {
    let mut term = Terminal::new(24, 80, 1000);
    term.osc_dispatch(&[b"9", b"Build complete!"], false);
    let notif = term.take_notification();
    assert!(notif.is_some());
    let (title, body) = notif.unwrap();
    assert_eq!(title, "SDIT");
    assert_eq!(body, "Build complete!");
}

#[test]
fn osc_99_notification() {
    let mut term = Terminal::new(24, 80, 1000);
    term.osc_dispatch(&[b"99", b"Task finished"], false);
    let notif = term.take_notification();
    assert!(notif.is_some());
    let (_, body) = notif.unwrap();
    assert_eq!(body, "Task finished");
}

#[test]
fn osc_9_notification_too_long() {
    let mut term = Terminal::new(24, 80, 1000);
    let long_body = "A".repeat(10000);
    term.osc_dispatch(&[b"9", long_body.as_bytes()], false);
    let notif = term.take_notification();
    assert!(notif.is_some());
    let (_, body) = notif.unwrap();
    assert!(body.len() <= 4096);
}

#[test]
fn osc_9_no_body_no_notification() {
    // パラメータが1つだけ（body なし）の場合は通知なし
    let mut term = Terminal::new(24, 80, 1000);
    term.osc_dispatch(&[b"9"], false);
    assert!(term.take_notification().is_none());
}

#[test]
fn take_notification_clears_pending() {
    let mut term = Terminal::new(24, 80, 1000);
    term.osc_dispatch(&[b"9", b"once"], false);
    let first = term.take_notification();
    let second = term.take_notification();
    assert!(first.is_some());
    assert!(second.is_none());
}

#[test]
fn osc_9_notification_sanitizes_control_chars() {
    let mut term = Terminal::new(24, 80, 1000);
    term.osc_dispatch(&[b"9", b"Hello\x00\x01\x07World"], false);
    let (_, body) = term.take_notification().unwrap();
    assert!(!body.contains('\x00'));
    assert!(!body.contains('\x01'));
    assert!(!body.contains('\x07'));
    assert!(body.contains("Hello"));
    assert!(body.contains("World"));
}

#[test]
fn osc_9_notification_truncates_utf8_safely() {
    let mut term = Terminal::new(24, 80, 1000);
    // 日本語テキスト（3バイト/文字）を大量に送って切り詰める
    let long_body = "あ".repeat(2000); // 6000 bytes > 4096
    term.osc_dispatch(&[b"9", long_body.as_bytes()], false);
    let (_, body) = term.take_notification().unwrap();
    assert!(body.len() <= 4096);
    // UTF-8 として有効であることを確認
    assert!(std::str::from_utf8(body.as_bytes()).is_ok());
}

// ---------------------------------------------------------------------------
// OSC 133 シェルインテグレーション テスト
// ---------------------------------------------------------------------------

// OSC 133;A でプロンプト開始マーカーが記録される
#[test]
fn osc_133_prompt_start() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b]133;A\x07");
    assert_eq!(term.semantic_markers.len(), 1);
    assert_eq!(term.semantic_markers[0].zone, SemanticZone::PromptStart);
}

// OSC 133;B でコマンド入力開始マーカーが記録される
#[test]
fn osc_133_command_start() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b]133;B\x07");
    assert_eq!(term.semantic_markers.len(), 1);
    assert_eq!(term.semantic_markers[0].zone, SemanticZone::CommandStart);
}

// OSC 133;C でコマンド出力開始マーカーが記録される
#[test]
fn osc_133_output_start() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    proc.advance(&mut term, b"\x1b]133;C\x07");
    assert_eq!(term.semantic_markers.len(), 1);
    assert_eq!(term.semantic_markers[0].zone, SemanticZone::OutputStart);
}

// OSC 133;D;0 でコマンド終了マーカー（終了コード付き）が記録される
#[test]
fn osc_133_command_end_with_exit_code() {
    // vte の osc_dispatch を直接呼んで検証（vte パーサー経由では ';' でパラメータ分割される）
    let mut term = Terminal::new(24, 80, 100);
    term.osc_dispatch(&[b"133", b"D", b"0"], false);
    assert_eq!(term.semantic_markers.len(), 1);
    assert_eq!(term.semantic_markers[0].zone, SemanticZone::CommandEnd(Some(0)));
}

// OSC 133;D（終了コードなし）でコマンド終了マーカーが記録される
#[test]
fn osc_133_command_end_no_exit_code() {
    let mut term = Terminal::new(24, 80, 100);
    term.osc_dispatch(&[b"133", b"D"], false);
    assert_eq!(term.semantic_markers.len(), 1);
    assert_eq!(term.semantic_markers[0].zone, SemanticZone::CommandEnd(None));
}

// prev_prompt / next_prompt のナビゲーション
#[test]
fn prompt_navigation() {
    let mut term = Terminal::new(24, 80, 100);
    // 行0, 5, 10 にプロンプトマーカーを配置
    term.semantic_markers.push_back(SemanticMarker { line: 0, zone: SemanticZone::PromptStart });
    term.semantic_markers.push_back(SemanticMarker { line: 5, zone: SemanticZone::PromptStart });
    term.semantic_markers.push_back(SemanticMarker { line: 10, zone: SemanticZone::PromptStart });
    // カーソルを行7に移動
    term.grid.cursor.point.line = Line(7);
    // prev_prompt → 行5
    assert_eq!(term.prev_prompt(), Some(5));
    // next_prompt → 行10
    assert_eq!(term.next_prompt(), Some(10));
}

// MAX_SEMANTIC_MARKERS を超えた場合に古いマーカーが削除される
#[test]
fn semantic_markers_capped() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    for _ in 0..10_001 {
        proc.advance(&mut term, b"\x1b]133;A\x07");
    }
    assert!(term.semantic_markers.len() <= 10_000);
}

// shell_integration_enabled = false のとき OSC 133 マーカーは記録されない
#[test]
fn osc_133_disabled_when_shell_integration_off() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    term.shell_integration_enabled = false;
    proc.advance(&mut term, b"\x1b]133;A\x07");
    assert_eq!(term.semantic_markers.len(), 0);
}

// CommandStart → CommandEnd で command_finished_pending がセットされる
#[test]
fn osc_133_command_finished_tracking() {
    let mut term = Terminal::new(24, 80, 100);
    // CommandStart で時間計測開始
    term.osc_dispatch(&[b"133", b"B"], false);
    assert!(term.command_start_time.is_some());
    assert!(term.command_finished_pending.is_none());
    // CommandEnd で command_finished_pending がセットされる
    term.osc_dispatch(&[b"133", b"D", b"0"], false);
    assert!(term.command_start_time.is_none());
    let pending = term.command_finished_pending.take();
    assert!(pending.is_some());
    let (elapsed_secs, exit_code) = pending.unwrap();
    assert_eq!(exit_code, Some(0));
    // 即座に終了したコマンドは 0 秒（elapsed < 1）
    assert_eq!(elapsed_secs, 0);
}

// CommandStart なしの CommandEnd は command_finished_pending が None のまま
#[test]
fn osc_133_command_finished_no_start() {
    let mut term = Terminal::new(24, 80, 100);
    // CommandStart なしで CommandEnd
    term.osc_dispatch(&[b"133", b"D", b"1"], false);
    assert!(term.command_finished_pending.is_none());
    assert!(term.command_start_time.is_none());
}

// NotificationConfig のデフォルト値の検証
#[test]
fn command_notify_config_defaults() {
    use crate::config::{CommandNotifyMode, NotificationConfig};
    let cfg = NotificationConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.command_notify, CommandNotifyMode::Unfocused);
    assert_eq!(cfg.command_notify_threshold, 10);
}

// command_notify_threshold のクランプ検証
#[test]
fn command_notify_config_threshold_clamp() {
    use crate::config::NotificationConfig;
    let cfg_zero =
        NotificationConfig { command_notify_threshold: 0, ..NotificationConfig::default() };
    assert_eq!(cfg_zero.clamped_command_notify_threshold(), 1);
    let cfg_over =
        NotificationConfig { command_notify_threshold: 9999, ..NotificationConfig::default() };
    assert_eq!(cfg_over.clamped_command_notify_threshold(), 3600);
    let cfg_normal =
        NotificationConfig { command_notify_threshold: 30, ..NotificationConfig::default() };
    assert_eq!(cfg_normal.clamped_command_notify_threshold(), 30);
}

// CommandNotifyMode のデシリアライズ検証
#[test]
fn command_notify_mode_deserialize() {
    use crate::config::{CommandNotifyMode, NotificationConfig};
    // TOML で command_notify フィールドをデシリアライズして検証
    let cfg_never: NotificationConfig = toml::from_str("command_notify = \"never\"").unwrap();
    assert_eq!(cfg_never.command_notify, CommandNotifyMode::Never);
    let cfg_unfocused: NotificationConfig =
        toml::from_str("command_notify = \"unfocused\"").unwrap();
    assert_eq!(cfg_unfocused.command_notify, CommandNotifyMode::Unfocused);
    let cfg_always: NotificationConfig = toml::from_str("command_notify = \"always\"").unwrap();
    assert_eq!(cfg_always.command_notify, CommandNotifyMode::Always);
}

// ---------------------------------------------------------------------------
// Phase 20.4: OSC Color Report Format
// ---------------------------------------------------------------------------

/// OscColorReportFormat のデフォルト値が SixteenBit であることを確認する。
#[test]
fn osc_color_report_format_default() {
    use crate::config::OscColorReportFormat;
    let term = Terminal::new(24, 80, 100);
    assert_eq!(term.osc_color_report_format, OscColorReportFormat::SixteenBit);
}

/// OSC 10;? に対して 16-bit 形式で応答することを確認する。
#[test]
fn osc_10_foreground_query_16bit() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // デフォルトは SixteenBit
    proc.advance(&mut term, b"\x1b]10;?\x1b\\");
    let writes = term.drain_pending_writes().unwrap_or_default();
    let response = std::str::from_utf8(&writes).unwrap();
    assert!(
        response.contains("rgb:ffff/ffff/ffff"),
        "expected 16-bit foreground response, got: {response:?}"
    );
}

/// OSC 11;? に対して 8-bit 形式で応答することを確認する。
#[test]
fn osc_11_background_query_8bit() {
    use crate::config::OscColorReportFormat;
    let (mut proc, mut term) = make_proc_term(24, 80);
    term.osc_color_report_format = OscColorReportFormat::EightBit;
    proc.advance(&mut term, b"\x1b]11;?\x1b\\");
    let writes = term.drain_pending_writes().unwrap_or_default();
    let response = std::str::from_utf8(&writes).unwrap();
    assert!(
        response.contains("rgb:00/00/00"),
        "expected 8-bit background response, got: {response:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 20.5: Title Report Flag
// ---------------------------------------------------------------------------

/// title_report のデフォルト値が false であることを確認する。
#[test]
fn title_report_config_default() {
    let term = Terminal::new(24, 80, 100);
    assert!(!term.title_report);
}

/// title_report=true かつタイトル設定済みのとき CSI 21 t で応答があることを確認する。
#[test]
fn csi_21t_title_report_enabled() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    term.title_report = true;
    // OSC 2 でタイトルを設定
    proc.advance(&mut term, b"\x1b]2;MyTitle\x1b\\");
    // CSI 21 t で問い合わせ
    proc.advance(&mut term, b"\x1b[21t");
    let writes = term.drain_pending_writes().unwrap_or_default();
    assert!(!writes.is_empty(), "expected title report response");
    let response = std::str::from_utf8(&writes).unwrap();
    assert!(response.contains("MyTitle"), "expected title in response, got: {response:?}");
}

/// title_report=false のとき CSI 21 t で応答しないことを確認する。
#[test]
fn csi_21t_title_report_disabled() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // title_report はデフォルト false
    proc.advance(&mut term, b"\x1b]2;MyTitle\x1b\\");
    proc.advance(&mut term, b"\x1b[21t");
    let writes = term.drain_pending_writes();
    assert!(writes.is_none(), "expected no response when title_report is disabled");
}

// ---------------------------------------------------------------------------
// Phase 20.6: Enquiry Response
// ---------------------------------------------------------------------------

/// enquiry_response のデフォルト値が None であることを確認する。
#[test]
fn enquiry_response_config_default() {
    let term = Terminal::new(24, 80, 100);
    assert!(term.enquiry_response.is_none());
}

/// enquiry_response が設定されているとき ENQ (0x05) で応答することを確認する。
#[test]
fn enq_response_set() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    term.enquiry_response = Some("test-answerback".to_string());
    proc.advance(&mut term, b"\x05");
    let writes = term.drain_pending_writes().unwrap_or_default();
    assert_eq!(std::str::from_utf8(&writes).unwrap(), "test-answerback");
}

/// enquiry_response=None のとき ENQ で応答しないことを確認する。
#[test]
fn enq_response_none() {
    let (mut proc, mut term) = make_proc_term(24, 80);
    // enquiry_response はデフォルト None
    proc.advance(&mut term, b"\x05");
    let writes = term.drain_pending_writes();
    assert!(writes.is_none(), "expected no response when enquiry_response is None");
}
