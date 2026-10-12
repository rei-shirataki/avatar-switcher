---
scope: api
title: VRChat REST API — タグ一覧
source: https://vrchat.community/tags
status: community
last_verified: 2026-10-09
---

# VRChat REST API — タグ一覧

ユーザー・ワールド・グループ・インスタンス・カレンダーイベントなどに付く `tags`（文字列配列）の意味をまとめたものです。出典は [vrchat.community/tags](https://vrchat.community/tags) で、**コミュニティによる解析**のため網羅的ではなく、予告なく変わります。

## 命名規則

- `admin_` で始まるタグは、**スタッフが手動で**付与する
- `system_` で始まるタグは、**システムが自動で**付与する
- それ以外は `author_tag_*`（作者付与）、`content_*`、`feature_*`、`language_*`、`permission-*` など用途別の接頭辞を持つ
- ユーザーのタグは `addTags` / `removeTags`（[user.md](user.md)）、ワールドは `addWorldTags` / `removeWorldTags`（[world.md](world.md)）で操作する。`admin_` / `system_` 系は一般ユーザーが付けられない

## ユーザータグ

| タグ | 意味 |
|---|---|
| `admin_avatar_access` | トラストランクなしでアバターをアップロードできる |
| `admin_can_grant_licenses` | 他ユーザーにライセンスを付与できる |
| `admin_canny_access` | トラストランクなしで Canny にアクセスできる |
| `admin_lock_tags` | ユーザーのタグがロックされ、本人が編集できない |
| `admin_lock_level` | トラストランクがロックされ、自動では変わらない |
| `admin_moderator` | VRChatスタッフ |
| `admin_official_thumbnail` | プロフィール画像をVRChatのロゴに置き換える |
| `admin_scripting_access` / `system_scripting_access` | ユーザー作成スクリプトのアップロード（非推奨） |
| `admin_world_access` | トラストランクなしでワールドをアップロードできる |
| `permission-upload-props` | Prop をアップロードできる |
| `permission-test-props` | アップロードした Prop をテストできる |
| `permission-platform-ios` | 不明 |
| `show_social_rank` | 実際のソーシャルランク表示の切り替え（非推奨。現在はレジストリキーでPhoton経由送信） |
| `show_mod_tag` | 赤いスタッフ用ネームプレートの表示切り替え |
| `system_avatar_access` | アバターのアップロードと公開が可能 |
| `system_early_adopter` | VRC+ のローンチ初期（2020年12月頃）に購入した |
| `system_feedback_access` | フィードバックを送信できる |
| `system_generated` | システムが内部的に作成したユーザー（2025-09-29に一時的に可視化、翌日削除） |
| `system_probable_troll` | 複数回通報されており、トロールの可能性が高い |
| `system_supporter` | VRC+ を購読中 |
| `system_troll` | 確認済みのトロール |
| `system_legend` | 2018年夏に活動していた経験豊富なプレイヤー（2022-05-05に削除） |
| `system_trust_basic` | New User（青）ランク |
| `system_trust_known` | User（緑）ランク |
| `system_trust_trusted` | Known User（オレンジ）ランク |
| `system_trust_veteran` | Trusted User（紫）ランク |
| `system_trust_intermediate` / `system_trust_advanced` | 不明（2022-05-05に削除） |
| `system_trust_legend` | Veteran User（金）ランク（2018年9月に廃止、タグは2022-05-05に削除） |
| `system_world_access` | ワールドのアップロードと公開が可能 |
| `system_vital_test_user_do_not_delete` | テスト・内部用の削除禁止ユーザー（2025-09-29に一時的に可視化、翌日削除） |

> **トラストランクのタグ名は1つずれています**: `system_trust_trusted` は実際には **Known User**（オレンジ）です。レガシーな命名のためで、上の表のとおり読み替えてください。Visitor にはタグがなく、User 以上でも New User のタグが無いことがよくあります。

## ワールドタグ

### 作者が付与するタグ

| タグ | 意味 |
|---|---|
| `author_tag_*` | 作者が付けるカスタムタグ（末尾は任意。タグ検索に使われる） |
| `author_tag_avatar` | 「Avatar Worlds」行に表示される |
| `author_tag_game` | 「Games」行に表示される |
| `content_adult` / `content_combat` / `content_featured` / `content_gore` / `content_horror` / `content_other` / `content_sex` / `content_violence` | コンテンツ警告（個別の説明は未記載） |

### 自動で付くタグ

| タグ | 意味 |
|---|---|
| `system_approved` | Community Labs を通じて自動承認された |
| `system_labs` | Community Labs に提出された |
| `system_created_recently` | 最近作成された |
| `system_updated_recently` | 最近更新された（`system_approved` も付いていると「Updated Recently」行に出る） |
| `system_published_recently` / `system_jam_*` / `system_monetized_world` / `system_positive_fun_to_explore` | 公開直後、ジャム参加、収益化ワールド、「Positive Fun to Explore」などに対応するとみられるタグ（説明は未記載） |
| `debug_allowed` | 作者がワールドデバッグを有効化した（トリガーの状態を確認できる） |

### スタッフが付与するタグ

| タグ | 意味 |
|---|---|
| `admin_featured` | 「Featured」カテゴリに載る |
| `admin_approved` | 手動で承認済み |
| `admin_avatar_world` | アバターワールドとして選定（非推奨） |
| `admin_community_spotlight` | 「Spotlight」行に載る |
| `admin_hidden` | 検索から常に除外される |
| `admin_hide_active` / `admin_hide_new` / `admin_hide_popular` | 「Active」／「Recently Updated Worlds」／「Popular Worlds」行から除外 |

上記以外にも、`admin_AllowInternal_*`（内部機能の許可。`Avatars`、`OpenMenu`、`UserSettings` など）、`admin_filter_*`、`admin_spotlight_*`、`admin_disable_avatar_collision`、`admin_disable_avatar_stations`、`admin_allow_master_abdication`、`admin_alternate_size_limits` などが存在します（説明は未記載）。

### 機能の無効化タグ（`feature_*`）

`feature_avatar_scaling` / `feature_avatar_scaling_disabled` / `feature_drones_disabled` / `feature_emoji_disabled` / `feature_focus_view_disabled` / `feature_pedestals_disabled` / `feature_prints_disabled` / `feature_props_disabled` / `feature_stickers_disabled` / `feature_third_person_view_disabled`

アバタースケール関連のタグは [OSC のアバタースケール](../osc/avatar-scaling.md)（`/avatar/eyeheightscalingallowed`）の挙動にも関係します。

### イベントタグ

ワールドにはイベント専用のタグも付きます（網羅はされていません）。例: `admin_vrrat_community_takeover`（Vket行）、`admin_muzzfesst`（MUZZFEST）、`admin_halloween_2019`（2019 Halloween行）、`admin_spookality_2024_featured`（Spookality）、`admin_pride2026`（Pride 2026）、`admin_tanabata2026`（Tanabata 2026）。

> **注意**: `admin_url_consent_furality_somna` は2025年の Furality Somna イベントで使われたタグで、Furality のブックマーク機能が個人情報を外部（`api.fynn.ai`）に送信することをユーザーに知らせる同意画面を、ワールド入場前に表示します。

## グループタグ

| タグ | 意味 |
|---|---|
| `admin_age_verification_enabled` | 年齢確認ベータの対象グループ（機能の正式公開に伴い非推奨） |
| `admin_hide_member_count` | オンラインメンバー数と総メンバー数を非表示にする（スタッフグループ `VRCHAT.0000` で使用） |
| `admin_featured_events_enabled` | 承認されたグループがカレンダーイベント作成時に `featured` フラグを使える |
| `admin_vrc_event_group_fair_enabled` | 承認されたグループが `vrc_event_group_fair` のカレンダーイベントタグを使える |

## 言語タグ（`language_*`）

ISO 639-3 の言語コードに基づくタグです。主にユーザープロフィールに付き、**1人あたり最大3つ**です。インスタンス（そこで話されている言語。`languages` / `languageRatio`）やカレンダーイベント（対象言語。**最大3つ**）にも付きます。

| タグ | 自称（Endonym） | 英語名 |
|---|---|---|
| `language_afr` | Afrikaans | Afrikaans |
| `language_ara` | العربية | Arabic |
| `language_ase` | American Sign Language | American Sign Language |
| `language_asf` | Auslan (Australian Sign Language) | Australian Sign Language |
| `language_ben` | বাংলা | Bengali |
| `language_bfi` | British Sign Language | British Sign Language |
| `language_bul` | български | Bulgarian |
| `language_ces` | Čeština | Czech |
| `language_cmn` | 官话 | Mandarin Chinese |
| `language_cym` | Cymraeg | Welsh |
| `language_dan` | Dansk | Danish |
| `language_deu` | Deutsch | German |
| `language_dse` | Nederlandse Gebarentaal | Dutch Sign Language |
| `language_ell` | Ελληνικά | Modern Greek (1453-) |
| `language_eng` | English | English |
| `language_epo` | Esperanto | Esperanto |
| `language_est` | eesti | Estonian |
| `language_fil` | Filipino | Filipino, Pilipino |
| `language_fin` | Suomi | Finnish |
| `language_fra` | Français | French |
| `language_fsl` | langue des signes française | French Sign Language |
| `language_gla` | Gàidhlig | Scottish Gaelic, Gaelic |
| `language_gle` | Gaeilge | Irish |
| `language_gsg` | Deutsche Gebärdensprache | German Sign Language |
| `language_heb` | עברית | Hebrew |
| `language_hin` | हिन्दी | Hindi |
| `language_hmn` | Hmoob | Hmong, Mong |
| `language_hrv` | hrvatski | Croatian |
| `language_hun` | Magyar | Hungarian |
| `language_hye` | հայերեն | Armenian |
| `language_ind` | Bahasa Indonesia | Indonesian |
| `language_isl` | íslenska | Icelandic |
| `language_ita` | Italiano | Italian |
| `language_jpn` | 日本語 | Japanese |
| `language_jsl` | 日本手話 | Japanese Sign Language |
| `language_kor` | 한국어 | Korean |
| `language_kvk` | 한국 수화 언어 | Korean Sign Language |
| `language_lav` | Latviešu | Latvian |
| `language_lit` | lietuvių | Lithuanian |
| `language_ltz` | Lëtzebuergesch | Luxembourgish, Letzeburgesch |
| `language_mar` | मराठी | Marathi |
| `language_mkd` | македонски | Macedonian |
| `language_mlt` | Malti | Maltese |
| `language_mri` | Māori | Maori |
| `language_msa` | Bahasa Melayu | Malay (macrolanguage) |
| `language_nld` | Nederlands | Dutch, Flemish |
| `language_nor` | Norsk | Norwegian |
| `language_nzs` | New Zealand Sign Language | New Zealand Sign Language |
| `language_pol` | Polski | Polish |
| `language_por` | Português | Portuguese |
| `language_ron` | Română | Romanian, Moldavian, Moldovan |
| `language_rus` | Русский | Russian |
| `language_sco` | Scots | Scots |
| `language_slk` | slovenčina | Slovak |
| `language_slv` | slovenščina | Slovenian |
| `language_spa` | Español | Spanish, Castilian |
| `language_swe` | Svenska | Swedish |
| `language_tel` | తెలుగు | Telugu |
| `language_tha` | ภาษาไทย | Thai |
| `language_tok` | toki pona | Toki Pona |
| `language_tur` | Türkçe | Turkish |
| `language_tws` | 潮州話 | Tio-Sua |
| `language_ukr` | украї́нська | Ukrainian |
| `language_vie` | Tiếng Việt | Vietnamese |
| `language_wuu` | 吳語 | Wu Chinese |
| `language_yue` | 廣東話 | Yue Chinese |
| `language_zho` | 中文 | Chinese |
| `language_zxx` | No linguistic content | No linguistic content, Not applicable |
