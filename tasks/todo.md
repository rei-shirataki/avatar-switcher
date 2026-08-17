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

## 設計メモ

プレハブ身長はアバター毎に不変。よって「リセット押下時に最新値同士を割る」のではなく、
「整合の取れた瞬間に一度計算してキャッシュし、ユーザ操作中は据え置く」のが根本解。
`isSteadyState()` が _lastEcho / _scaleFactor / _scaleModified の更新条件を統一することで
ペアの整合性を構造的に保証する。
