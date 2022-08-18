special-patch
====

謎のソースコードの書き換えツール

現在の機能：
- `NULL` を `(NULL)` に置換する


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
cargo run ~/shina-lab/project-ultimate-sanitizer/magma-v1.2/targets/openssl/repo/compile_commands.json
```