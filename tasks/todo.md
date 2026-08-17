# /code-review 指摘修正（2026-06-10）

身長リセット機能（プレハブ身長逆算）のコードレビューで確定した 7 件のうち 6 件を修正。

## タスク

- [x] 1. 時間的不整合: `getAvatarDefault()` が異時点の `_lastEcho` / `_scaleFactor` を除算 → 定常状態（非スムージング・エコー抑止外・切替残響外）で受信した整合ペアから `_prefabHeight` を事前計算してキャッシュする方式へ変更
- [x] 2. アバター切替後の残留 race → `AVATAR_SETTLE_MS`(300ms) の残響窓を導入し、窓内の Scale 系メッセージをプレハブ計算入力から除外
- [x] 3. `osc:avatar-change` リスナー登録失敗の無言握り潰し → `_lastError` へ表面化（eye-height リスナーと同パターン）
- [x] 4. Rust `ScaleModified` の `OscType::Bool` 厳格マッチ → Bool / Int(0/1) / Float(0.0/1.0) を寛容に受理
- [x] 5. 数値強制変換パターンの 4 箇所重複 → `src/app/core/utils/number.util.ts` の `coerceFiniteNumber()` に集約
- [x] 6. UnlistenFn 4 フィールドの解放ボイラープレート（約20行）→ `unlisteners: UnlistenFn[]` ＋ループ解放
- [ ] 7. avatar.service / eye-height.service の 'osc:avatar-change' 二重購読 → 検証で「疎結合設計として妥当」と判定されたため対応せず（意図的スキップ）

## 検証結果（Artifacts）

- `npm run build` … 成功（Application bundle generation complete, 5.5s）
- `cargo check` … 成功（Finished dev profile, 1.17s）
- 付随効果: applyExternalValue のガード順を元に戻したため、抑止中の不要な normalize/localStorage 書き込み懸念も解消

## /code-review 指摘修正（2026-08-17, Issue #1）

`eye-height.service.ts` に対する `/code-review` バックグラウンドエージェントの指摘2件を修正。

### タスク

- [x] 1. `recomputePrefabHeight()` の scale 状態不整合 → `_lastEcho` / `_scaleFactor` / `_scaleModified` それぞれに受信時刻 (`_lastEchoAt` 等) を持たせ、`SCALE_PAIR_WINDOW_MS`(250ms) 以内に届いた組だけを「同一時点の観測ペア」として prefab height の計算に採用するよう変更。ペアが揃わない間は直前の正しいキャッシュ値を据え置く（＝自己修復性を保ったまま、遠く離れた時刻の値同士が誤ってペアリングされるのを防止）
- [x] 2. コンストラクタの `listen()` unlisten 登録漏れ → `destroyed` フラグ＋ `registerListener()` ヘルパーに統一。`onDestroy` が `listen()` の Promise 解決より先に発火した場合、解決後に unlisten を即座に呼んでリークを防ぐ

### 検証（手動シナリオトレース、レース系のため自動テストなし）

- シナリオ1a（ScaleModifiedのみ登録・VRC本体でリサイズ）: ロード時 `_lastEcho=1.7@t0`, `_scaleModified=false@t0` でペア成立 → `_prefabHeight=1.7`。数秒後リサイズで `_lastEcho=2.5@t1` に更新されるが `_scaleModifiedAt` は依然 `t0`（数秒前）のため `|t1-t0|≥250ms` でペア不成立 → キャッシュは `1.7` のまま据え置き。直後に `ScaleModified=true@t2` が届いても `=== false` を満たさないため何もしない。修正前は誤って `2.5` に上書きされ二度と直らなかった箇所が、修正後は正しい値を維持する。
- シナリオ1b（ScaleFactor併用・echoが境界を跨いで到着）: 数分前の古い `_lastEcho` に直近の新しい `_scaleFactor` をペアリングしようとしても `250ms` を超えるためスキップ。直後に新しい `EyeHeight` echo が届けば近接時刻で正しくペア成立し自己修復（修正前と同じ自己修復性を維持）。
- リスナーリーク: `registerListener()` で `destroyed` チェックを追加。`npx tsc --noEmit -p tsconfig.app.json` … エラーなし。`npm run build` … 成功（Application bundle generation complete, 5.81s）。

## 設計メモ

プレハブ身長はアバター毎に不変。よって「リセット押下時に最新値同士を割る」のではなく、
「整合の取れた瞬間に一度計算してキャッシュし、ユーザ操作中は据え置く」のが根本解。
`isSteadyState()` が _lastEcho / _scaleFactor / _scaleModified の更新条件を統一することで
ペアの整合性を構造的に保証する。
