/** 小数として受け付ける文字列（全体一致）。"1.7abc" や "1 2" のような
 *  先頭だけ数値の文字列を parseFloat のように部分的に読まないため。 */
const DECIMAL_PATTERN = /^[+-]?(\d+(\.\d*)?|\.\d+)$/;

/** 全角数字・全角記号（IME 入力）を半角にし、小数点のカンマをドットにそろえる。
 *  "1,75" を parseFloat すると 1 になり、身長が 1.00m に設定されてしまうため。 */
function normalizeDecimalText(text: string): string {
  return text
    .trim()
    .replace(/[０-９]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 0xfee0))
    .replace(/[．。]/g, '.')
    .replace(/[，、,]/g, '.')
    .replace(/[－−]/g, '-')
    .replace(/＋/g, '+');
}

/** unknown 値を有限数値へ強制変換する。変換できなければ null。
 *  OSC ペイロードやフォーム入力など「number のはずだが型保証がない」値の
 *  受け口を 1 箇所に集約する。NaN / Infinity / 数値として全体を読めない文字列はすべて null。 */
export function coerceFiniteNumber(raw: unknown): number | null {
  if (typeof raw === 'number') return Number.isFinite(raw) ? raw : null;
  if (typeof raw !== 'string') return null;
  const text = normalizeDecimalText(raw);
  if (!DECIMAL_PATTERN.test(text)) return null;
  const v = Number(text);
  return Number.isFinite(v) ? v : null;
}
