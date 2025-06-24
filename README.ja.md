# priority-semaphore 日本語版

Rust 用のランタイム非依存な優先度付き非同期セマフォです。

タスクは署名付きの優先度を指定してパーミットを取得できます。数値が大きい
ほど高優先度とみなされ、先にウェイクされます。重要な処理を優先しつつ、
飢餓状態を避けることができます。

## 特徴

- **Tokio** または **async-std** を選択して利用可能
- キャンセルセーフな `acquire`
- `ageing` フィーチャによる簡易エージング戦略
- `unsafe` コードゼロ

## 使用例

```rust
use std::sync::Arc;
use priority_semaphore::PrioritySemaphore;

#[tokio::main]
async fn main() {
    let sem = Arc::new(PrioritySemaphore::new(1));

    let hi = sem.clone();
    let h = tokio::spawn(async move {
        let _permit = hi.acquire(10).await.unwrap();
        println!("high priority job");
    });

    let lo = sem.clone();
    let l = tokio::spawn(async move {
        let _permit = lo.acquire(1).await.unwrap();
        println!("low priority job");
    });

    h.await.unwrap();
    l.await.unwrap();
}
```

詳細な例は [`examples`](./examples) ディレクトリを参照してください。

## クレートのフィーチャ

| フィーチャ | 既定 | 説明                                     |
|-----------|------|------------------------------------------|
| `tokio`   | ✔    | Tokio ランタイム用サポート               |
| `async-std` | ❌ | async-std 用サポート                      |
| `ageing`  | ❌   | 飢餓状態緩和のための簡易エージング       |
| `std`     | ✔    | 標準ライブラリを使用                     |
| `docsrs`  | ❌  | docs.rs ビルド用の内部フィーチャ          |

## ライセンス

このプロジェクトは MIT または Apache License 2.0 のいずれかを
選択して使用できます。
