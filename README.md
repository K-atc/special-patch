special-patch
====

謎のソースコードの書き換えツール

現在の機能：
- `NULL` を `(NULL)` に置換する
- プリプロセッサが出力したソースコードに置き換える（`--preprocessor` オプション）


How to install
----
```shell
cargo install --git https://github.com/K-atc/special-patch.git --bins --all-features
```

Or manually git clone and:

```shell
cargo install --path . --bins --all-features
```


How to use
----
```shell
git -C ../magma-v1.2/targets/openssl/repo reset --hard && cargo run -- --preprocessor ../magma-v1.2/targets/openssl/repo/compile_commands.json
```