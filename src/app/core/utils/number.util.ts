/** unknown 値を有限数値へ強制変換する。変換できなければ null。
 *  OSC ペイロードやフォーム入力など「number のはずだが型保証がない」値の
 *  受け口を 1 箇所に集約する。NaN / Infinity / 非数値文字列はすべて null。 */
export function coerceFiniteNumber(raw: unknown): number | null {
  const v = typeof raw === 'number' ? raw : parseFloat(String(raw));
  return Number.isFinite(v) ? v : null;
}
