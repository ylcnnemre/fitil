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

1. **Başlat:** Eski süreç kapatılır, uygulama başlatılır: `Start-Process target\debug\fitil.exe`. Çalışan bir `fitil.exe` varken `cargo build` dosyanın üzerine yazamaz.
2. **Görüntü al:** `scripts/capture-window.ps1 -OutFile <yol>.png` çalıştırılır.
   - Betik PrintWindow kullanır: Fitil penceresi başka pencerelerin arkasında olsa da görüntü alınır.
   - Proje sahibi o sırada bilgisayarı kullanıyor olabilir. Pencereyi öne getirmek veya gerçek fareyi oynatmak yasak.
3. **İncele:** PNG dosyası Read aracıyla açılır. Görüntü dosyası depoya değil, oturumun scratchpad klasörüne yazılır.

**Tıklama yok:** iced, PostMessage ile gönderilen tıklamaları kabul etmiyor. Bu yüzden tıklama gerektiren akışları proje sahibi dener. Mantık birim testleri ve canlı testlerle doğrulanır.

**Spot sembol listesi yavaş:** Debug derlemede spot exchangeInfo büyük olduğu için açılışta yaklaşık 10–15 saniye sürüyor. Release derlemede çok daha hızlı.

## Binance notları

- **REST adresleri:**
  - Spot: `https://api.binance.com/api/v3`, istek ağırlığı limiti 6000/dk.
  - Vadeli: `https://fapi.binance.com/fapi/v1`, limit 2400/dk.
  - Limitler exchangeInfo'dan okunur.
- **WebSocket adresleri:**
  - Spot: `wss://stream.binance.com:9443/stream`.
  - Vadeli: `wss://fstream.binance.com/market/stream`. 2026-04-23'ten beri `/market` yolu şart. Derinlik ve bookTicker akışları ise `/public` yolunda.
- **İstek ağırlıkları:**
  - Spot: exchangeInfo 20, tüm tickerlar 80, klines 2.
  - Vadeli: exchangeInfo 1, tickerlar 40, klines (limit 1000) 5.
- **`!ticker@arr` yok:** Spot'ta 2026-03-26'da emekliye ayrıldı. Tüm semboller için `!miniTicker@arr` kullanılır.
- **Mum akışı:** Güncelleme sadece işlem olunca gelir. Açık mum ayrıca REST'ten çekilir.
- **Mum sayfalama:** Sadece `endTime` verilirse o zamana kadarki son `limit` mum döner. Dönen mum sayısı `limit`ten azsa geçmişin başına ulaşılmıştır.

## Canlı testler

İnternet gerektiren testler varsayılan olarak atlanır. Elle şöyle çalıştırılır:

```bash
cargo test -p fitil-exchange -- --ignored   # REST + WebSocket, iki piyasa
cargo test -p fitil-app -- --ignored        # veri katmanı uçtan uca
```

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

## Kullanılan kütüphane sürümleri

| Kütüphane | Sürüm | Not |
|---|---|---|
| iced | 0.14 | `tokio` özelliği açık |
| fluent-bundle | 0.16 | |
| unic-langid | 0.9 | |
| sys-locale | 0.3 | |
| tokio | 1 | |
| reqwest | 0.13 | `default-features = false`, `rustls-no-provider` |
| rustls | 0.23 | Sadece `ring`. aws-lc lisans sebebiyle kullanılmıyor. Sağlayıcı `fitil_exchange::install_crypto_provider` ile kuruluyor. |
| tokio-tungstenite | 0.30 | `rustls-tls-native-roots` |
| rusqlite | 0.41 | `bundled`: SQLite kaynaktan derleniyor |
| serde, serde_json | 1 | |
| thiserror | 2 | |
| chrono | 0.4 | Yerel saat dilimi için |
| directories | 6 | Uygulama klasörleri için |
| futures | 0.3 | |

## iced 0.14 notları

- **Başlatma:** `iced::application(State::new, State::update, State::view)`. `new` fonksiyonu `(State, Task<Message>)` döndürür.
- **Builder:** `.title(...)`, `.theme(...)`, `.subscription(...)`, `.window_size(...)` ve `.run()` builder üzerinden çağrılır.
- **Uzun süren işler:** `Subscription::run_with(data, fn(&D) -> Stream)` ile çalıştırılır. Örnek: `main.rs` içindeki `live_feed`, `iced::stream::channel` ile kuruluyor ve kendi komut kanalını `Message::FeedReady` olarak geri gönderiyor. Fonksiyon `impl Stream + use<>` döndürmeli.
- **Stil fonksiyonları:**
  - `button::primary`, `button::secondary`, `button::text`
  - `text::success`, `text::danger`, `text::secondary`
  - `rule::horizontal(1)`, `space().width(Fill)`, `iced::padding::bottom(10)`
  - Padding için 4 elemanlı dizi kabul edilmiyor.
- **Kaynak kodu:** iced'in kaynağı `~/.cargo/registry/src/*/iced_*-0.14*` altında. API'den şüphe edilirse oraya bakılır.

## Dikkat edilecekler

- **Lisans:** GPL-3.0. Eklenen her bağımlılık uyumlu olmalı, `deny.toml` bunu otomatik kontrol eder.
  - `cargo deny check` şu an "license was not encountered" uyarıları veriyor. Bunlar izin listesinde olup henüz kullanılmayan lisanslar için, hata değil.
  - TLS için `rustls` ile `ring` kullanılacak. aws-lc'den kaçınılır.
- **Gizlilik:** Depo herkese açık. Gizli bilgi commit edilmez.
- **Kod dili:** Kod, kod içi açıklamalar ve commit mesajları İngilizce. Belgeler ve `memory/` Türkçe.
