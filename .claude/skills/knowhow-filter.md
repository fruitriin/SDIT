---
name: knowhow-filter
description: Plan ファイルの内容を受け取り、docs/knowhow/ から関連するノウハウのパスと要約だけを返す。ブートシーケンスだけでなく開発中いつでも利用してよい。
context: fork
user_invocable: true
---

# Knowhow フィルタリング

Plan の内容に基づいて、`docs/knowhow/` 内のノウハウファイルから関連するものだけを選別して返す。

## 引数
- `$ARGUMENTS`: Plan ファイルのパス（例: `docs/plans/phase6.1-mouse-scroll.md`）

## 手順

1. `$ARGUMENTS` で指定された Plan ファイルを読む
2. `docs/knowhow/` 内の全 `.md` ファイル（`feature-survey-log.md` を除く）を読む
3. Plan の実装に **必要または有用** なノウハウを判定する
4. 以下の形式で結果を返す:

```
## 関連ノウハウ

### docs/knowhow/xxx.md
要約: （1〜2文で内容を要約）
関連理由: （Plan のどの部分に関係するか）

### docs/knowhow/yyy.md
要約: ...
関連理由: ...
```

5. 関連するノウハウがない場合は「関連するノウハウはありません」と返す

## 判定基準
- Plan で扱う技術領域（VTE, PTY, wgpu, winit, grid 等）に直接関係するか
- Plan の実装で注意すべきハマりポイントが記載されているか
- Plan のアーキテクチャ判断に影響する知見があるか
