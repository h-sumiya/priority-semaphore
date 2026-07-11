# priority-semaphore 日本語版

Rust 向けの高速・ランタイム非依存な優先度付き非同期セマフォです。

取得要求ごとに `i32` の優先度を指定します。値が大きい待機者へ次のパーミットを
割り当て、同じ優先度では FIFO 順に処理します。

## 設計上の特徴

- 非競合時は atomic のみを使うロックフリー高速パス
- 競合時は選択した待機者へパーミットを直接ハンドオフし、新規タスクによる横取りを防止
- 世代付きインデックスヒープにより、追加／キャンセルは O(log n)、Waker 更新は O(1)
- 直接割り当ての前後を含めた完全なキャンセルセーフ
- `close`、パーミット返却、キュー登録を線形化可能な形で同期
- Tokio、async-std、smol、独自 executor のどれでも利用できるランタイム非依存設計
- `std` と `no_std + alloc` の両方でスレッドセーフ
- このクレート内の unsafe コードはゼロ

## インストール

```toml
[dependencies]
priority-semaphore = "0.2.0"
```

## 使用例

```rust
use priority_semaphore::PrioritySemaphore;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let semaphore = Arc::new(PrioritySemaphore::new(8));

    let permit = semaphore.acquire(100).await.unwrap();
    // 優先処理を実行
    drop(permit); // 最も優先度の高い待機者へ返却
}
```

RAII パーミットがセマフォを所有するため、`acquire` は
`Arc<PrioritySemaphore>` に対して呼び出します。取得 Future はいつ drop しても安全です。
`try_acquire` は、より大きな優先度を渡しても既存の待機者を追い越しません。

実行可能な Example:

```console
cargo run --example priority
cargo run --example cancellation
cargo run --example try_acquire
```

## 動作仕様

- 大きい `i32` 値ほど高優先度です。
- 同一優先度は FIFO 順です。
- 優先度が影響するのはキューでの待機時だけです。
- 厳密な優先度制御なので、高優先度処理が流入し続けると低優先度処理は待ち続ける場合があります。
- `close()` 後の新規取得は失敗し、キュー内の Future は `AcquireError::Closed` で起床します。
  close 前に割り当て済み／取得済みのパーミットは有効です。
- panic やタスクキャンセルを含め、パーミットは `Drop` で必ず返却されます。

## フィーチャ

| フィーチャ | 既定 | 説明 |
| --- | --- | --- |
| `std` | 有効 | 短いキュー操作に `parking_lot` を使用 |
| `docsrs` | 無効 | docs.rs 用設定 |

`std` を無効にすると短いスピン Mutex を利用します。この構成でもスレッド間共有は安全です。
クレートは `alloc` を必要とします。

## 検証とベンチマーク

直接ハンドオフの競合、割り当て前後のキャンセル、close／返却／キャンセルの同時実行、
優先度と FIFO、8 スレッドでの継続的な高負荷をテストしています。

参考値として、ローカルの x86_64 環境で release ビルドを計測したところ、非競合の
取得／返却は約 **15.4 ns**（同じベンチマークの Tokio owned permit は約
**24.1 ns**）、優先度付きの競合ハンドオフは毎秒約 **115 万件**でした。
結果はハードウェアやワークロードで変動します。また、Tokio は FIFO、本クレートは
優先度順という仕様差があるため、比較値は参考情報です。

```console
cargo test --all-features
cargo test --release --all-features
cargo bench --bench throughput
```

## ライセンス

MIT または Apache-2.0 のいずれかを選択できます。
