# Contributing to Fitil

> **Türkçe özet:** Katkıların için teşekkürler! Büyük bir değişiklikten önce bir issue açıp konuş. Kod ve kod içi açıklamalar İngilizce yazılır. Arayüz metinleri `locales/` klasöründeki dosyalardan gelir. Yeni bir dil eklemek için aşağıdaki "Translations" bölümüne bak, Rust bilmen gerekmez. Fitil'e al-sat veya emir gönderme özelliği eklenmeyecek.

Thanks for your interest in Fitil! This guide explains how to get started.

## Before you start

- **Discuss first.** For anything bigger than a small fix, please open an issue before writing code, so we can agree on the approach.
- **Read the design.** The product scope is defined in [docs/TASARIM.md](docs/TASARIM.md). It is written in Turkish; feel free to ask questions in English in an issue.
- **Respect the scope.** Fitil is an analysis tool. Features that place orders, trade, or need exchange API keys will not be accepted.

## Development setup

1. Install [Rust](https://rustup.rs) (stable). On Windows, also install the Microsoft C++ build tools when the installer asks.
2. Clone the repository and run the app:

   ```bash
   cargo run -p fitil-app
   ```

## Checks

Every pull request must pass these, and CI runs them on Windows, macOS and Linux:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check        # install with: cargo install cargo-deny --locked
```

## Code guidelines

- **Language:** Code, comments and commit messages are in English.
- **No hard-coded UI text:** Every string a user sees comes from the Fluent files in `locales/`. Do not hard-code text in Rust code.
- **Dependencies:** New dependencies must have a license compatible with GPL-3.0. `cargo deny check` enforces this. Keep the dependency tree small.

## Translations

You can add a language without knowing Rust. If you prefer, open an issue with the translated file and a maintainer will wire it up.

1. Copy `locales/en/fitil.ftl` to `locales/<code>/fitil.ftl`, where `<code>` is the language code (for example `de` or `fr`).
2. Translate the text on the right side of each `=`. Keep the keys and `{ $placeholders }` unchanged.
3. Add a `language-<code>` entry with the language's own name to **every** `fitil.ftl` file.
4. In `crates/fitil-app/src/i18n.rs`, add a variant to `Language` and extend `ALL`, `code`, `name_key` and `source`.
5. Run `cargo test`. A test checks that every language defines exactly the same keys.

## Pull request review

Every pull request is reviewed before it is merged. Changes that touch networking, file access, build scripts, CI or dependencies get extra scrutiny, because Fitil is downloaded by many people.

## Reporting security issues

Please do not open a public issue for security problems. Use GitHub's private vulnerability reporting instead: open the "Security" tab of the repository and choose "Report a vulnerability".

## License

By contributing, you agree that your contributions are licensed under the [GPL-3.0](LICENSE).
