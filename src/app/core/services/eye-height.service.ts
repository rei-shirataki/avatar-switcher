import { DestroyRef, Injectable, computed, inject, signal } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TauriService } from './tauri.service';
import { coerceFiniteNumber } from '../utils/number.util';

const STORAGE_KEY_VALUE = 'avatar-switcher.eyeheight.value';
const STORAGE_KEY_MODE = 'avatar-switcher.eyeheight.mode';

export const EYE_HEIGHT_DEFAULT = 1.6;
export const EYE_HEIGHT_MIN = 0.2;
export const EYE_HEIGHT_MAX = 5.0;

/** 自前の OSC 送信後、この期間内に VRChat から届くエコーは applyExternalValue で無視する。
 *  スムーズ終端で _isSmoothing=false に戻った直後に届く中間値エコーが _target を
 *  巻き戻すレースを潰す。VRChat 側の echo 遅延 (~50–200ms) と SMOOTH_DURATION_MS の
 *  どちらも余裕で覆える値にしてある。 */
const ECHO_SUPPRESS_MS = 800;

/** アバター切替通知（/avatar/change）の直後、この期間内に届く Scale 系メッセージは
 *  旧アバターの送信残り（UDP バッファ遅延）の可能性があるため、プレハブ身長の
 *  計算入力としては捨てる。新アバターのパラメータダンプはロード完了後
 *  （通常 1 秒以上先）に届くので、この窓で取りこぼすことはまずない。 */
const AVATAR_SETTLE_MS = 300;

/** EyeHeight と ScaleFactor / ScaleModified は独立した OSC ストリームで届くため、
 *  「両方とも steady state で受信済み」なだけでは同一時点の観測とは限らない
 *  （例: 数分前の EyeHeight エコーに、たった今届いた ScaleFactor をペアリングして
 *  しまう）。両フィールドの受信時刻差がこのウィンドウ内に収まる場合のみ「同じ
 *  瞬間の観測ペア」とみなして prefab height の計算に使う。パラメータダンプや
 *  リサイズ操作は同一バーストで数十ms 以内に複数パラメータが届くのに対し、
 *  無関係な単発更新は秒〜分単位で離れるため、この閾値で十分弁別できる。 */
const SCALE_PAIR_WINDOW_MS = 250;

/** 送信モード。どちらも公式 `/avatar/eyeheight` に Float(m) を送る。
 *  - `instant`: 目標値を 1 発で送る。VRChat 側で即座に反映。
 *  - `smooth` : 現在値→目標値を ~500ms でイージング補間し、~33ms 毎に小刻みに
 *               送信して見た目をスムーズに変化させる。 */
export type EyeHeightMode = 'instant' | 'smooth';
const DEFAULT_MODE: EyeHeightMode = 'instant';
const SMOOTH_DURATION_MS = 500;
const SMOOTH_STEP_MS = 33;

/** スムーズモードのイージング曲線。easeInOutCubic 相当の対称カーブ。
 *  立ち上がりはゆっくり → 中盤で加速 → 終端で減速、の S 字を描く。 */
const SMOOTH_EASE = cubicBezier(0.65, 0.0, 0.35, 1.0);

/** CSS の cubic-bezier(x1,y1,x2,y2) と同じ曲線を返すイージング関数を生成する。
 *  返り値は (t: 0〜1) → (eased: 0〜1)。
 *  x(u) = t の u を Newton 法で解いて y(u) を返す標準実装。 */
function cubicBezier(x1: number, y1: number, x2: number, y2: number): (t: number) => number {
  const cx = 3 * x1;
  const bx = 3 * (x2 - x1) - cx;
  const ax = 1 - cx - bx;
  const cy = 3 * y1;
  const by = 3 * (y2 - y1) - cy;
  const ay = 1 - cy - by;
  const sampleX = (u: number) => ((ax * u + bx) * u + cx) * u;
  const sampleY = (u: number) => ((ay * u + by) * u + cy) * u;
  const derivX = (u: number) => (3 * ax * u + 2 * bx) * u + cx;
  return (t: number) => {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    let u = t;
    for (let i = 0; i < 8; i++) {
      const dx = sampleX(u) - t;
      if (Math.abs(dx) < 1e-6) break;
      const slope = derivX(u);
      if (Math.abs(slope) < 1e-6) break;
      u -= dx / slope;
    }
    return sampleY(u);
  };
}

@Injectable({ providedIn: 'root' })
export class EyeHeightService {
  private readonly _value = signal<number>(EYE_HEIGHT_DEFAULT);
  /** 直近の正規化済み目標値。adjust() の起点・スムーズ中の表示に使う。
   *  スムージング中はまだ補間途上で _value() < target なので、起点を _value()
   *  にすると連打時の +δ が壊れるためここで別途保持する。 */
  private readonly _target = signal<number>(EYE_HEIGHT_DEFAULT);
  /** スムーズ補間が進行中か。UI 表示の出し分けに使う signal。 */
  private readonly _isSmoothing = signal<boolean>(false);
  private readonly _mode = signal<EyeHeightMode>(DEFAULT_MODE);
  private readonly _lastError = signal<string | null>(null);
  /** ワールド(Udon)が公開する `/avatar/eyeheightmin` / `max`。未受信ならアプリの
   *  デフォルト範囲にフォールバックする。公式仕様上 OSC 書き込みはこの範囲の
   *  制限を受けないため、送信のクランプには使わず UI 上の目安表示にのみ使う。
   *  `/avatar/change` 受信時に null へリセットする（onAvatarChange 参照）。
   *  ワールド境界そのものを検知する OSC シグナルは存在しないため、アバター
   *  ロードのたびに発火する avatar-change を代替トリガーとして使う。 */
  private readonly _worldMinHeight = signal<number | null>(null);
  private readonly _worldMaxHeight = signal<number | null>(null);
  /** `/avatar/eyeheightscalingallowed`。false のとき VRChat 側は `/avatar/eyeheight`
   *  への書き込みを無言で無視するため、送信自体は止めずユーザーに可視化する。
   *  null（未受信）と false を区別し、警告は false のときだけ出す。 */
  private readonly _scalingAllowed = signal<boolean | null>(null);

  readonly value = this._value.asReadonly();
  readonly target = this._target.asReadonly();
  readonly isSmoothing = this._isSmoothing.asReadonly();
  readonly mode = this._mode.asReadonly();
  readonly lastError = this._lastError.asReadonly();
  // アプリの静的クランプ(EYE_HEIGHT_MIN/MAX)とワールドの範囲、より厳しい方を
  // 採用する。normalize() は静的クランプしかかけないため、UI がこれより緩い
  // 範囲を受理すると「入力は通ったのに送信時に無言で丸められる」ズレが起きる。
  readonly worldMinHeight = computed(() => Math.max(this._worldMinHeight() ?? EYE_HEIGHT_MIN, EYE_HEIGHT_MIN));
  readonly worldMaxHeight = computed(() => Math.min(this._worldMaxHeight() ?? EYE_HEIGHT_MAX, EYE_HEIGHT_MAX));
  /** null = 未受信（不明）。false のときだけ警告表示に使う。 */
  readonly scalingAllowed = this._scalingAllowed.asReadonly();

  /** 登録済み Tauri イベントリスナーの解放関数。onDestroy で一括解放する。 */
  private readonly unlisteners: UnlistenFn[] = [];
  /** onDestroy 済みか。listen() の Promise 解決が onDestroy より後になった場合、
   *  unlisteners への push ではなく即座に unlisten してリークを防ぐために参照する。 */
  private destroyed = false;
  /** 進行中のスムージング interval ハンドル。null なら未進行。 */
  private smoothTimer: ReturnType<typeof setInterval> | null = null;
  /** 直近で sendOsc を呼んだ performance.now() 時刻。エコー抑止の起点。 */
  private _lastSelfSendAt = 0;

  // ── アバター本来の身長（プレハブ Eye Height）の逆算用 ──
  //
  // VRChat の Built-in Parameters `ScaleFactor` / `ScaleModified` がアバターの
  // Animator Playable Layer に登録されている場合に限り送られてくる。
  // 揃えば `EyeHeightAsMeters / ScaleFactor = PrefabHeight`、
  // または `ScaleModified === false` なら EyeHeight 自身がプレハブ身長。
  //
  // EyeHeight と ScaleFactor は独立した OSC ストリームで届くため、「最新値同士」を
  // その場で除算すると異なる時点の値をペアにしてしまい誤った身長を返し得る
  // （スムーズ補間中のエコーと遅延した ScaleFactor の組み合わせ等）。そこで
  // 計算入力は isSteadyState()（自前送信の影響もアバター切替の残響もない状態）で
  // 受信したものに限定した上で、さらに SCALE_PAIR_WINDOW_MS 以内に届いた組だけを
  // 「同一時点の観測ペア」とみなして _prefabHeight を計算・キャッシュする
  // （isSteadyState を満たすタイミングは各ストリームで独立なので、それだけでは
  // 数分前の値と直近の値が混ざるのを防げないため）。プレハブ身長はアバター毎に
  // 一定なので、整合ペアが得られない間は直前の正しいキャッシュ値をそのまま使う。
  //
  // アバター切替時は全状態をクリアする（古いアバターの ScaleFactor が新アバターで
  // 残ると誤計算する）。非対応アバターでは null のままなので EYE_HEIGHT_DEFAULT に
  // fallback する。
  private _scaleFactor: number | null = null;
  private _scaleModified: boolean | null = null;
  /** 定常状態で観測した VRChat 由来の最新 EyeHeight。プレハブ身長の計算入力。 */
  private _lastEcho: number | null = null;
  // 各フィールドを最後に更新した performance.now() 時刻。SCALE_PAIR_WINDOW_MS で
  // 「同じ瞬間の観測ペアか」を判定するために使う。未受信は -Infinity のままにして
  // おけば、差分が必ず SCALE_PAIR_WINDOW_MS を超えるので自然にペア対象から外れる。
  private _lastEchoAt = Number.NEGATIVE_INFINITY;
  private _scaleFactorAt = Number.NEGATIVE_INFINITY;
  private _scaleModifiedAt = Number.NEGATIVE_INFINITY;
  /** 整合ペアから逆算したプレハブ身長のキャッシュ。未確定なら null。 */
  private _prefabHeight: number | null = null;
  /** 直近の /avatar/change 受信時刻。AVATAR_SETTLE_MS の起点。 */
  private _lastAvatarChangeAt = Number.NEGATIVE_INFINITY;

  constructor(private tauri: TauriService) {
    const saved = parseFloat(localStorage.getItem(STORAGE_KEY_VALUE) ?? '');
    if (Number.isFinite(saved)) {
      this._value.set(this.normalize(saved));
    }
    this._target.set(this._value());
    const savedMode = localStorage.getItem(STORAGE_KEY_MODE);
    if (savedMode === 'instant' || savedMode === 'smooth') {
      this._mode.set(savedMode);
    }
    // VRChat からの現在アイハイト通知を購読する。
    // サービスは providedIn: 'root' で常駐するため、別ビューにいてアバター切替された
    // 場合でも EyeHeightAsMeters を取りこぼさずに最新値を保持できる。
    this.registerListener(
      listen<number>('osc:eye-height', e => this.applyExternalValue(e.payload)),
      err => this._lastError.set(`OSC受信購読失敗: ${String(err)}`),
    );

    // Avatar Scaling Built-in Parameters。届かないアバターも多いが、
    // 届いている間はプレハブ身長を正確に逆算できる。
    // この 2 つは取れなくても EYE_HEIGHT_DEFAULT への fallback で動くので握り潰す。
    this.registerListener(listen<number>('osc:scale-factor', e => this.applyScaleFactor(e.payload)));
    this.registerListener(listen<boolean>('osc:scale-modified', e => this.applyScaleModified(e.payload)));

    // アバター切替時はスケール状態を必ずリセットする。新アバターが ScaleFactor を
    // 公開していない場合、古い値を引きずると別アバターの倍率で誤計算してしまう。
    // この購読が失敗するとリセット漏れ＝誤計算に直結するため、エラーは UI に出す。
    this.registerListener(
      listen<string>('osc:avatar-change', () => this.onAvatarChange()),
      err => this._lastError.set(`OSC受信購読失敗 (avatar-change): ${String(err)}`),
    );

    // ワールド(Udon)が公開する範囲・書き込み許可。届かないワールドも多く、
    // 未受信でもアプリのデフォルト範囲で動作するため握り潰す。
    // アバター切替直後 AVATAR_SETTLE_MS 以内の受信は旧ワールド/旧アバターの
    // 送信残り（UDP バッファ遅延）の可能性があるため、onAvatarChange でクリア
    // した直後に古い値で上書きしないよう捨てる（isSteadyState は使わない。
    // このトリオは自前送信もスムーズ補間もしないため、その判定は無関係な
    // ECHO_SUPPRESS_MS 中の受信まで誤って捨ててしまう）。
    this.registerListener(listen<number>('osc:eye-height-min', e => {
      const v = coerceFiniteNumber(e.payload);
      if (v !== null && !this.isAvatarSettling()) this._worldMinHeight.set(v);
    }));
    this.registerListener(listen<number>('osc:eye-height-max', e => {
      const v = coerceFiniteNumber(e.payload);
      if (v !== null && !this.isAvatarSettling()) this._worldMaxHeight.set(v);
    }));
    this.registerListener(listen<boolean>('osc:eye-height-scaling-allowed', e => {
      if (!this.isAvatarSettling()) this._scalingAllowed.set(e.payload);
    }));

    // dev HMR 等でサービスが再構築される際にリスナーと interval を確実に解放する。
    // providedIn:'root' でも DestroyRef は機能する。
    inject(DestroyRef).onDestroy(() => {
      this.destroyed = true;
      this.cancelSmooth();
      for (const un of this.unlisteners.splice(0)) un();
    });
  }

  /** listen() の Promise を登録し、解決した unlisten を保持する。
   *  onDestroy が Promise 解決より先に発火した場合（HMR 等）は push せず即座に
   *  unlisten を呼び、リスナーリークを防ぐ。 */
  private registerListener(pending: Promise<UnlistenFn>, onError?: (err: unknown) => void): void {
    pending
      .then(un => {
        if (this.destroyed) {
          un();
          return;
        }
        this.unlisteners.push(un);
      })
      .catch(err => onError?.(err));
  }

  /** モードを切り替えて永続化する。進行中のスムージングはキャンセル。 */
  setMode(mode: EyeHeightMode): void {
    this.cancelSmooth();
    this._mode.set(mode);
    localStorage.setItem(STORAGE_KEY_MODE, mode);
  }

  /** ユーザ操作で値をセットし VRChat へ送信する。
   *  smooth モード時は現在値→目標値を補間しながら連続送信する。
   *  途中で新しい setValue が来たらその時点の現在値から再補間。 */
  async setValue(v: number): Promise<void> {
    if (!Number.isFinite(v)) return;
    const target = this.normalize(v);
    this.cancelSmooth();
    this._target.set(target);
    if (this._mode() === 'smooth') {
      this.startSmoothTo(target);
    } else {
      await this.commit(target);
    }
  }

  /** スムージング中なら停止する。新しい目標が来たとき・モード変更時に呼ぶ。 */
  private cancelSmooth(): void {
    if (this.smoothTimer !== null) {
      clearInterval(this.smoothTimer);
      this.smoothTimer = null;
    }
    this._isSmoothing.set(false);
  }

  /** 現在値から target まで SMOOTH_DURATION_MS 掛けてイージング補間しながら送信。
   *  途中の step は **丸めない生 Float** を OSC で送ることで VRChat 側の見た目を
   *  なめらかに連続変化させる。表示は displayValue() の toFixed(2) で別途整える。
   *  終端だけ正規化済み target に着地させて 0.01 単位できれいに止める。 */
  private startSmoothTo(target: number): void {
    const start = this._value();
    // 差分が 0.01 未満なら補間せず即時送信して終了。
    if (Math.abs(target - start) < 0.005) {
      void this.commit(target);
      return;
    }
    const startTime = performance.now();
    this._isSmoothing.set(true);
    this.smoothTimer = setInterval(() => {
      const elapsed = performance.now() - startTime;
      const t = Math.min(1, elapsed / SMOOTH_DURATION_MS);
      if (t >= 1) {
        // 終端：正規化済み target で完全停止し、永続化もここで。
        this.cancelSmooth();
        this._value.set(target);
        void this.sendOsc(target);
        this.persistValue(target);
        return;
      }
      // 途中：生 Float のまま流す（clamp だけかける、丸めない）。
      const eased = SMOOTH_EASE(t);
      const raw = this.clamp(start + (target - start) * eased);
      this._value.set(raw);
      void this.sendOsc(raw);
    }, SMOOTH_STEP_MS);
  }


  /** 即時送信＋永続化。instant モードと smooth 終端で使う共通パス。 */
  private async commit(value: number): Promise<void> {
    this._value.set(value);
    this.persistValue(value);
    await this.sendOsc(value);
  }

  private async sendOsc(value: number): Promise<void> {
    // エコー抑止ウィンドウを送信ごとに更新する。Promise resolve を待たず、
    // 「発火した瞬間」から VRChat エコーが戻り得るのでここで打刻する。
    this._lastSelfSendAt = performance.now();
    try {
      await this.tauri.invoke('osc_set_avatar_eye_height', { value });
      this._lastError.set(null);
    } catch (e) {
      this._lastError.set(`OSC 送信失敗: ${String(e)}`);
    }
  }

  private persistValue(value: number): void {
    localStorage.setItem(STORAGE_KEY_VALUE, String(value));
  }

  /** 増減ボタン用：起点は補間中の中間値ではなく _target（直近の目標値）。
   *  これでスムーズモードで連打しても期待通りの累計分だけ動く。 */
  async adjust(delta: number): Promise<void> {
    await this.setValue(this._target() + delta);
  }

  /** VRChat から受け取った値を再送なしで反映する。
   *  生の float（例: 1.6000001）が来るので、ボタン操作と同じ 0.01 桁に
   *  正規化して格納し、表示と次回 adjust() の起点が常にクリーンに保たれるようにする。
   *  スムージング進行中、および直近の自前送信から ECHO_SUPPRESS_MS 以内のエコーは
   *  「自分が投げた値が戻ってきただけ」とみなして無視する。スムーズ終端直後の
   *  遅延エコーで _target が中間値に巻き戻る race を防ぐ。 */
  private applyExternalValue(raw: unknown): void {
    const v = coerceFiniteNumber(raw);
    if (v === null) return;
    if (this._isSmoothing()) return;
    if (performance.now() - this._lastSelfSendAt < ECHO_SUPPRESS_MS) return;
    const normalized = this.normalize(v);
    // ここまで来た時点で自前送信の影響はないが、アバター切替直後の残響だけは
    // 別途除外してプレハブ身長の計算入力に採用する。
    if (this.isSteadyState()) {
      this._lastEcho = normalized;
      this._lastEchoAt = performance.now();
      this.recomputePrefabHeight();
    }
    this._value.set(normalized);
    // VRC 側で身長を変えた直後にユーザがボタンを押した時、その新しい値を
    // 起点にして増減できるよう _target も同期しておく。
    this._target.set(normalized);
    this.persistValue(normalized);
  }

  private applyScaleFactor(raw: unknown): void {
    const v = coerceFiniteNumber(raw);
    if (v === null || v <= 0) return;
    // スムーズ補間中・エコー抑止中に届く ScaleFactor は自前送信による変化の途中値で、
    // 抑止されている _lastEcho とペアが揃わないため捨てる。最後に観測した整合ペア
    // から計算済みの _prefabHeight がそのまま有効（プレハブ身長は不変）。
    if (!this.isSteadyState()) return;
    this._scaleFactor = v;
    this._scaleFactorAt = performance.now();
    this.recomputePrefabHeight();
  }

  private applyScaleModified(raw: unknown): void {
    if (typeof raw !== 'boolean') return;
    if (!this.isSteadyState()) return;
    this._scaleModified = raw;
    this._scaleModifiedAt = performance.now();
    this.recomputePrefabHeight();
  }

  private onAvatarChange(): void {
    this._scaleFactor = null;
    this._scaleModified = null;
    this._lastEcho = null;
    this._prefabHeight = null;
    this._lastEchoAt = Number.NEGATIVE_INFINITY;
    this._scaleFactorAt = Number.NEGATIVE_INFINITY;
    this._scaleModifiedAt = Number.NEGATIVE_INFINITY;
    this._lastAvatarChangeAt = performance.now();
    // ワールド/Udon スコープのトリオもここでクリアする。avatar-change を「境界」の
    // 代替シグナルとして使う都合上、旧ワールドの値を次のワールド/アバターに
    // 引き継がないことを優先する（Issue #6）。新しい値は世界側が再送すれば
    // isAvatarSettling() の窓を抜けた直後に上書きされ、再送されないワールドでは
    // null（=アプリのデフォルト範囲・警告なし）にフォールバックする。
    this._worldMinHeight.set(null);
    this._worldMaxHeight.set(null);
    this._scalingAllowed.set(null);
  }

  /** アバター切替直後 AVATAR_SETTLE_MS 以内かどうか。旧アバター/旧ワールド由来の
   *  送信残り（UDP バッファ遅延）を弾くための共通窓。 */
  private isAvatarSettling(): boolean {
    return performance.now() - this._lastAvatarChangeAt < AVATAR_SETTLE_MS;
  }

  /** プレハブ身長の計算入力を受け入れてよい「定常状態」かどうか。
   *  自前送信の影響（スムーズ補間・エコー抑止窓）と、アバター切替直後の
   *  旧アバター由来メッセージの残響期間を除外する。_lastEcho / _scaleFactor /
   *  _scaleModified がすべて同じ条件で更新されることで、互いに整合の取れた
   *  ペアであることが保証される。 */
  private isSteadyState(): boolean {
    if (this._isSmoothing()) return false;
    if (performance.now() - this._lastSelfSendAt < ECHO_SUPPRESS_MS) return false;
    return !this.isAvatarSettling();
  }

  /** 定常状態で観測した整合ペアからプレハブ身長を再計算してキャッシュする。
   *  「isSteadyState() を満たした」だけでは同一時点の観測とは限らない
   *  （EyeHeight と ScaleFactor/ScaleModified は独立した OSC ストリーム）ため、
   *  SCALE_PAIR_WINDOW_MS 以内に届いたペアのみを信頼する。
   *
   *  優先順位:
   *  1. ScaleFactor が EyeHeight と近接時刻で届いていれば `EyeHeight / ScaleFactor`
   *     で逆算（最も正確）
   *  2. ScaleModified が EyeHeight と近接時刻で false と届いていれば、
   *     現在の EyeHeight 自身がプレハブ身長
   *  3. どちらも整合ペアが無ければ計算不能（キャッシュ据え置き。ペアが古いだけで
   *     プレハブ身長自体は不変なので、直前の正しいキャッシュ値を捨てない） */
  private recomputePrefabHeight(): void {
    if (this._lastEcho == null) return;
    if (this._scaleFactor != null && Math.abs(this._lastEchoAt - this._scaleFactorAt) < SCALE_PAIR_WINDOW_MS) {
      this._prefabHeight = this.normalize(this._lastEcho / this._scaleFactor);
    } else if (
      this._scaleModified === false &&
      Math.abs(this._lastEchoAt - this._scaleModifiedAt) < SCALE_PAIR_WINDOW_MS
    ) {
      this._prefabHeight = this.normalize(this._lastEcho);
    }
  }

  /** アバター本来のプレハブ身長を返す。リセットボタンが使用する。
   *  未確定（Avatar Scaling 非対応アバター・受信前）なら EYE_HEIGHT_DEFAULT。
   *  キャッシュは normalize 済みなので [EYE_HEIGHT_MIN, EYE_HEIGHT_MAX] に収まる。 */
  getAvatarDefault(): number {
    return this._prefabHeight ?? EYE_HEIGHT_DEFAULT;
  }

  /** 範囲クランプ＋0.01 桁丸めを 1 箇所に集約。
   *  すべての書き込みパスはこれを通すこと。 */
  private normalize(v: number): number {
    return Math.round(this.clamp(v) * 100) / 100;
  }

  private clamp(v: number): number {
    return Math.max(EYE_HEIGHT_MIN, Math.min(EYE_HEIGHT_MAX, v));
  }
}
