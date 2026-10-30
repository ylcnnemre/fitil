# Teknik Notlar

## Geliştirme bilgisayarı

- **İşletim sistemi:** Windows 11 Home.
- **Proje klasörü:** `C:\Users\ylcnn\Desktop\tradingclone`.
- **Git:** 2.54. GitHub CLI (`gh`) ile `ylcnnemre` hesabında oturum açık. Gerekli yetkiler mevcut: repo, workflow.
- **Paket yöneticisi:** winget mevcut.
- **Kurulan araçlar** (30 Ekim 2026, winget ile):
  - Microsoft C++ derleme araçları: Visual Studio Build Tools 2022, "VCTools" iş yükü ve önerilen bileşenler. Rust'ın Windows'ta (MSVC) bağlantı yapabilmesi için gerekli.
  - Rust: `Rustlang.Rustup`, kararlı (stable) sürüm.
- **PATH uyarısı:** Rust kurulduktan sonra açık olan terminallerde `cargo` bulunamayabilir. Bu durumda yeni bir terminal açılır ya da tam yol kullanılır: `~/.cargo/bin/cargo`. Claude'un Bash aracında komutların başına `export PATH="$HOME/.cargo/bin:$PATH";` eklenmeli.
- **Kurulu sürümler:** Rust 1.99.1, cargo-deny 0.20.2.
- **GitHub CLI:** Git Bash'te `gh api /...` gibi başında `/` olan yolları dosya yolu sanıyor. Baştaki `/` olmadan yazılmalı, örneğin `gh api licenses/gpl-3.0`.

## Arayüzü kontrol etmek

Claude ekranı doğrudan göremez. Arayüzde bir değişiklikten sonra:

1. Uygulama başlatılır: `Start-Process target\debug\fitil.exe`.
2. Pencerenin görüntüsü alınır: `scripts/capture-window.ps1 -OutFile <yol>.png`. Betik sadece Fitil penceresini yakalar.
3. PNG dosyası Read aracıyla açılıp incelenir. Görüntü dosyası depoya değil, oturumun scratchpad klasörüne yazılır.

## Sık kullanılan komutlar

```bash
cargo run -p fitil-app            # uygulamayı çalıştır
cargo test --workspace            # tüm testler
cargo fmt --all                   # kod biçimini düzelt
cargo clippy --workspace --all-targets -- -D warnings   # hata yakalayıcı
cargo deny check                  # bağımlılıkların lisans ve güvenlik kontrolü
```

## Proje yapısı

Bir Cargo workspace olarak kuruldu, ayrıntısı `docs/TASARIM.md` §6.1'de.

| Crate | Görevi |
|---|---|
| `crates/fitil-core` | Ortak veri tipleri |
| `crates/fitil-exchange` | Borsa adaptörleri |
| `crates/fitil-store` | SQLite |
| `crates/fitil-indicators` | İndikatör hesaplamaları |
| `crates/fitil-chart` | Grafik motoru |
| `crates/fitil-app` | Uygulamanın kendisi, iced arayüzü |
| `locales/<dil>/fitil.ftl` | Arayüz metinleri |

Yeni bir dil eklemek için `locales/` altına yeni bir klasör açılır, dosya çevrilir ve dil kodu `fitil-app` içindeki dil listesine eklenir.

## Kullanılan kütüphane sürümleri (Adım 1)

| Kütüphane | Sürüm |
|---|---|
| iced | 0.14 |
| fluent-bundle | 0.16 |
| unic-langid | 0.9 |
| sys-locale | 0.3 |

## iced 0.14 notları

- Uygulama şöyle başlatılır: `iced::application(State::new, State::update, State::view)`. `new` fonksiyonu `(State, Task<Message>)` döndürür.
- `.title(...)`, `.theme(...)`, `.window(...)` ve `.run()` builder üzerinden çağrılır.

## Dikkat edilecekler

- **Lisans:** GPL-3.0. Eklenen her bağımlılık uyumlu olmalı, `deny.toml` bunu otomatik kontrol eder.
  - `cargo deny check` şu an "license was not encountered" uyarıları veriyor. Bunlar izin listesinde olup henüz kullanılmayan lisanslar için, hata değil.
  - TLS için `rustls` ile `ring` kullanılacak. aws-lc'den kaçınılır.
- **Gizlilik:** Depo herkese açık. Gizli bilgi commit edilmez.
- **Kod dili:** Kod, kod içi açıklamalar ve commit mesajları İngilizce. Belgeler ve `memory/` Türkçe.
