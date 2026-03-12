# マウスイベント報告の実装知見

## マウスモード体系

| DECSET | モード | 内容 |
|--------|--------|------|
| ?9     | X10    | クリックのみ |
| ?1000  | X11    | クリック + リリース |
| ?1002  | Button-event | ドラッグ中のマウス移動も報告 |
| ?1003  | Any-event | 全マウス移動を報告（負荷大） |
| ?1005  | UTF-8  | 座標をUTF-8でエンコード（非推奨、未実装） |
| ?1006  | SGR    | `CSI < button ; col ; row M/m` 形式。座標上限なし |

## SGR vs X11 形式

- **SGR** (`?1006`): `\x1b[<button;col;row{M|m}` — 座標が1-based、リリースは `m`
- **X11** (デフォルト): `\x1b[M cb cx cy` — 各バイトに +32 でエンコード、座標上限 222

SGR が有効なら SGR 形式を使い、そうでなければ X11 形式にフォールバック。

## ボタンエンコーディング

- 0=左, 1=中, 2=右
- 32 を加算するとドラッグ中のボタン表現（例: 左ドラッグ = 32）
- 64=scroll up, 65=scroll down

## GUI スレッドからの送信経路

マウスレポートは `pending_writes` ではなく、キー入力と同じ `pty_io.write_tx.try_send()` で直接 PTY に送信する。
理由: マウスイベントは GUI スレッドで発生し、Terminal のロックを短時間取得してモード確認後すぐに送信する。

## セキュリティ: スクロール行数の上限

winit の `MouseScrollDelta::LineDelta` は理論上無制限の値が来る可能性がある。
ループで PTY に報告するため `.clamp(1, 20)` で上限を設け、DoS を防止。

## ビューポートスクロール

- マウスモード OFF 時: ホイールで `Grid::scroll_display(Scroll::Delta(lines))`
- Shift+PageUp/Down: 半画面分スクロール
- PTY 出力時: `display_offset > 0` なら `Scroll::Bottom` でリセット（ライブビュー追従）
