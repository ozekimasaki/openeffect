# OpenEffect

After Effects のプロジェクトを扱えるオープンソースのコンポジタ。

このリポジトリは共有契約だけを置く。

| クレート | 内容 |
| --- | --- |
| `crates/model` | プロジェクト、フォルダ、フッテージ、コンポジション、レイヤー、プロパティ、キーフレーム、マスク、シェイプ、テキスト |
| `crates/eval` | 時刻から値を取るサンプラと、合成入力のトレイト |
| `crates/effect-api` | パラメータ定義と、CPU / GPU の適用入口 |
| `crates/ui-api` | ドック、パネル、コマンド、ショートカット、ヒストリ |
| `crates/io-api` | フッテージ参照、未リンク、プロキシ、レンダーキューの出力 |
| `crates/three-d` | カメラ、ライト、マテリアル、クラシック 3D と Advanced 3D の計算 |

```
cargo check --workspace
```

ライセンスは Apache-2.0。
