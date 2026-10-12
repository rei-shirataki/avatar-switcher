/**
 * 例外からユーザー向けのメッセージを取り出す。Tauri の invoke は Rust 側の
 * `Err(String)` を Error ではなく文字列のまま reject するため、`instanceof Error`
 * だけで判定するとバックエンドのメッセージが常に捨てられてしまう。
 */
export function errorMessage(e: unknown, fallback: string): string {
  if (typeof e === 'string' && e) return e;
  if (e instanceof Error && e.message) return e.message;
  return fallback;
}
