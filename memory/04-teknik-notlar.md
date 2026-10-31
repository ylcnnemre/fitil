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

**Grafik açarak test:** `Start-Process target\debug\fitil.exe -ArgumentList "--open","binance:futures:ETHUSDT:45m"` grafiği tıklamadan açar. Biçim `[borsa:]piyasa:SEMBOL[:zaman dilimi]`, piyasa `spot` ya da `futures`.

**Kullanıcının penceresi açıkken test etmek:**
- `target\debug\fitil.exe` çalışırken `cargo build` ve `cargo test` dosyanın üzerine yazamaz.
- Pencereyi kapatmak yerine ayrı bir klasöre derle: `CARGO_TARGET_DIR=target/verify cargo test --workspace`. İlk derleme birkaç dakika sürer.
- Yeni sürümü kullanıcının önbelleğine dokunmadan açmak için önbelleği bir kopyaya yönlendir: `FITIL_DATA_DIR=<klasör>`. Eski sürüm açıkken yeni sürüm aynı dosyada göç (migration) çalıştırırsa eski sürüm hata verir.
- Test için açılan süreç `Start-Process -PassThru` ile alınan Id üzerinden kapatılır. Görüntü almak için `capture-window.ps1 -ProcessId <id>` kullanılır.

**Açık pencereye dikkat:** Proje sahibi o sırada uygulamayı kullanıyor olabilir. Ekran görüntüsünde imleç veya değişmiş bir görünüm varsa `fitil` sürecini kapatmadan önce sor ya da yeniden başlatmadan doğrula.

**`python3` kullanma:** Bu makinede `python3` komutu Windows Store'un yer tutucusu ve komutu sonsuza kadar bekletiyor. Betikler için `node` kullan (v22 kurulu).

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

Mimari "port ve adaptör" (hexagonal) düzenindedir. Kurallar ve bağımlılık tablosu `docs/TASARIM.md` §6'da, klasör yapısı §6.4'te.

| Crate | Görevi | İçerik |
|---|---|---|
| `fitil-core` | Alan modeli | `Candle` (`quote_volume`, `trades`, `taker_buy_volume` alanları `Option`), `ExchangeId`, `MarketKind` (Spot, LinearPerpetual), `Market` (= borsa + tür), `Symbol`, `Ticker`, `SeriesKey`, `Timeframe`, `aggregate`, `aggregate_whole` |
| `fitil-exchange` | Borsa arayüzü ve ortak araçlar | `Exchange` trait (async-trait), `CandleQuery` (Latest/From/Until), `Features`, `RequestUsage`, `ExchangeError` (Clone), feed tipleri (`Topic`, `FeedCommand`, `FeedEvent`), `stream::run_stream` ve `StreamProtocol`, `RateLimiter` (pencere süresi ayarlanır), `Clock`, `http_client()` |
| `fitil-binance` | Binance adaptörü | `rest.rs` (piyasa başına REST istemcisi, istek ağırlığı), `stream.rs` (`BinanceStream` protokolü), `wire.rs` (JSON). Kayıtlı yanıtlar `tests/fixtures/binance/`'te. |
| `fitil-store` | SQLite | Göç 1 (ilk şema), göç 2 (anahtara `exchange` eklendi, isteğe bağlı alanlar NULL olabilir, `usdm` → `perp`) |
| `fitil-market` | Uygulama katmanı | `loader.rs` (`open_series`, `load_older`, `save_candle`), `series.rs` (`Series`: bellekteki mumlar, canlı mum, `LiveUpdate`), `board.rs` (`Board`: fiyat listesi, dolar hacmine göre sıralama, `load_market`), `fake.rs` (testler için sahte borsa) |
| `fitil-indicators` | İndikatör hesapları | Şimdilik boş, Adım 4'te dolacak |
| `fitil-chart` | Grafik bileşeni | iced Canvas üzerinde, sıfırdan yazıldı |
| `fitil-app` | Pencere ve ekranlar | `main.rs` (durum, mesajlar, `exchanges()` borsa listesi, `--open`), `view.rs`, `feed.rs` (canlı veri aboneliği), `storage.rs`, `error.rs`, `format.rs`, `i18n.rs` |
| `locales/<dil>/fitil.ftl` | Arayüz metinleri | |

**Yeni dil:** `locales/` altına yeni bir klasör açılır, dosya çevrilir ve dil kodu `fitil-app/src/i18n.rs` içindeki dil listesine eklenir.

**Yeni borsa** (`docs/TASARIM.md` §6.3):
1. `crates/fitil-<borsa>` açılır ve `fitil_exchange::Exchange` uygulanır. Canlı veri için `StreamProtocol` yazılır, yeniden bağlanma döngüsü ortaktır.
2. Kayıtlı yanıtlarla testler ve `--ignored` canlı testler eklenir.
3. `fitil-app/src/main.rs` içindeki `exchanges()` listesine eklenir.
4. Workspace `Cargo.toml` dosyasına ve `fitil-app` bağımlılıklarına eklenir.

**Bağımlılık kuralı:** `fitil-market` hiçbir adaptörü tanımaz. Adaptörler `fitil-market`'i tanımaz. Uygulama hepsini birbirine bağlar.

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
- **Canvas (iced'in `canvas` özelliği açık):**
  - `canvas::Program::update` fonksiyonu `Option<Action<Message>>` döndürür: `Action::publish(m).and_capture()` ya da `Action::request_redraw()`.
  - Çizim için `Frame::fill_rectangle`, `stroke`, `fill_text` ve `with_clip(rect, |frame| …)` kullanılır.
  - Canvas kendi alanını kendisi kırpmaz. Taşabilecek her şey `with_clip` ile çizilmeli.
  - `canvas::Cache` boyut değişince kendiliğinden yeniden çizer, aksi hâlde `clear()` gerekir.
- **`iced::mouse` modülü Click yardımcısını dışarı açmıyor.** Çift tıklama `draw.rs` içinde zaman ve mesafe kontrolüyle algılanıyor.

## Grafik motoru (`fitil-chart`)

- **Durum:** `Chart` kendi canvas'ının programıdır (`impl canvas::Program<ChartMessage> for Chart`). Uygulama `chart.view().map(Message::Chart)` ile gösterir, gelen mesajı `chart.update(msg)` ile uygular.
- **Görünüm:** Görünüm çubuk sırası üzerinden tutulur. Zaman ekseni çubuk sırasıyla ilerler: boşluklar veya ay uzunlukları sorun olmaz.
- **Eski mumlar:** Eski mumlar `set_candles` ile tüm dizi verilerek eklenir. Grafik, eski ilk mumun yeni dizideki yerini bulup görünümü kaydırır.
- **Etiketler:** Grafiğin dil metinleri (`ChartLabels`) uygulamadan, Fluent'ten gelir. `chart-months` ve `chart-magnitudes` anahtarları virgülle ayrılmış listelerdir.
- **Renkler:** Yükselen #2EBD85, düşen #F6465D. Şimdilik sabit, Adım 7'de ayarlanabilir olacak.

## Dikkat edilecekler

- **Lisans:** GPL-3.0. Eklenen her bağımlılık uyumlu olmalı, `deny.toml` bunu otomatik kontrol eder.
  - `cargo deny check` şu an "license was not encountered" uyarıları veriyor. Bunlar izin listesinde olup henüz kullanılmayan lisanslar için, hata değil.
  - TLS için `rustls` ile `ring` kullanılacak. aws-lc'den kaçınılır.
- **Gizlilik:** Depo herkese açık. Gizli bilgi commit edilmez.
- **Kod dili:** Kod, kod içi açıklamalar ve commit mesajları İngilizce. Belgeler ve `memory/` Türkçe.

## Diğer borsalar (araştırma, 31 Ekim 2026)

Resmi belgelerden ve Türkiye IP'sinden yapılan canlı denemelerden derlendi. Adaptör yazarken bu notlarla başla, ama güncel belgeye mutlaka tekrar bak. Arayüze etkileri `docs/TASARIM.md` §6.2'deki "Borsalar arasındaki farklar" tablosunda.

**OKX** (https://www.okx.com/docs-v5/en/)
- **Semboller:** Spot `BTC-USDT`, doğrusal perpetual `BTC-USDT-SWAP` / `BTC-USDC-SWAP`.
- **Sembol listesi:** `GET /api/v5/public/instruments?instType=`. Fiyat hassasiyeti `tickSz`'den, kontrat büyüklüğü `ctVal`'den alınır.
- **Mumlar:**
  - `/market/candles` sadece son 1440 mumu verir, eskisi `/market/history-candles` ile alınır.
  - İstek başına en fazla 300 mum. Sonuçlar yeniden eskiye sıralı. `after=ts` daha eskileri, `before=ts` daha yenileri getirir.
  - Zaman dilimleri: `1m 3m 5m 15m 30m 1H 2H 4H`, UTC için `6Hutc 12Hutc 1Dutc 1Wutc 1Mutc`. Bunların `utc`siz hâlleri UTC+8'e hizalı.
  - Satır: `[ts,o,h,l,c,vol,volCcy,volCcyQuote,confirm]`. Taker hacmi ve işlem sayısı yok.
- **24 saat istatistiği:** `GET /market/tickers?instType=` (`last`, `open24h`, `volCcy24h`).
- **WebSocket:**
  - Fiyatlar `wss://ws.okx.com/ws/v5/public`, mumlar `wss://ws.okx.com/ws/v5/business` adresinde.
  - Tüm piyasa için tek bir fiyat kanalı yok. İstemci 30 saniyeden kısa aralıklarla `ping` metni gönderir.
- **Limitler:** Mumlar 40 istek / 2 sn, geçmiş mumlar 20 istek / 2 sn. Kullanım başlıkta bildirilmiyor.
- **Saat:** `GET /api/v5/public/time`.

**Hyperliquid** (https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api)
- **İstekler:** Hepsi `POST https://api.hyperliquid.xyz/info` adresine `{"type": …}` gövdesiyle gider.
- **Semboller:** Perpetual'lar coin adıyla (`BTC`), spot `PURR/USDC` ya da `@107` biçiminde.
- **Fiyat hassasiyeti:** Sabit fiyat adımı yok. 5 anlamlı basamak; en fazla `6-szDecimals` ondalık (spotta `8-szDecimals`).
- **Mumlar:**
  - İstek: `candleSnapshot {coin, interval, startTime, endTime}`.
  - Sadece son 5000 mum var. Eskiden yeniye sıralı. Alanlar `o h l c v n` (`n` = işlem sayısı). Taker hacmi ve kote hacmi yok.
  - Hizalama: `1w` perşembe başlıyor, `1M` 30 günlük. Bu ikisi Fitil'e doğrudan zaman dilimi olarak bildirilmemeli.
- **24 saat istatistiği:** `metaAndAssetCtxs` (`prevDayPx`, `dayNtlVlm`, `midPx`). Son işlem fiyatı yok.
- **WebSocket:**
  - Adres `wss://api.hyperliquid.xyz/ws`. Mum aboneliği: `{"method":"subscribe","subscription":{"type":"candle",…}}`.
  - Tüm piyasa kanalları: `allMids` ve `allDexsAssetCtxs`.
  - İstemci `{"method":"ping"}` gönderir. Sunucu 60 saniye sessiz kalırsa bağlantıyı kapatır.
- **Limitler:** IP başına dakikada 1200 ağırlık. Mum isteği 20 ağırlık, dönen mum sayısıyla artıyor.

**HTX / Huobi** (https://www.htx.com/en-us/opend/newApiPages/; eski huobiapi.github.io belgeleri güncel değil)
- **Semboller ve adresler:**
  - Spot `btcusdt` (`api.huobi.pro`).
  - USDT teminatlı perpetual `BTC-USDT` (`api.hbdm.com`).
- **Mumlar:**
  - Spot REST'te sadece son 2000 mum var, zaman parametresi yok. Daha eskisi WebSocket `req` isteğiyle alınır (`from`/`to`, istek başına 900 mum).
  - Günlük ve daha uzun mumlar UTC+8'e hizalı ve UTC seçeneği yok. Bu yüzden Fitil'e en fazla 4 saatlik zaman dilimleri bildirilmeli.
- **WebSocket:**
  - Her mesaj gzip ile sıkıştırılmış.
  - Sunucu 5 saniyede bir `{"ping":ms}` gönderir, istemci `{"pong":ms}` ile yanıt verir.
- **24 saat istatistiği:** Spot `/market/tickers` toplu veri verir ama açılış fiyatı UTC+8 gününe göre.
- **Diğer:** "Çin dışı IP" şartı var.

**Bybit v5** (https://bybit-exchange.github.io/docs/v5/)
- **Mumlar:**
  - `GET /v5/market/kline`: istek başına en fazla 1000, yeniden eskiye sıralı, `start`/`end` ms cinsinden.
  - Zaman dilimleri `1 3 5 15 30 60 120 240 360 720 D W M`, UTC'ye hizalı.
  - Satır `[start,o,h,l,c,volume,turnover]`.
- **24 saat istatistiği:** `GET /v5/market/tickers?category=` tüm semboller için tek istekte (`lastPrice`, `prevPrice24h`, `turnover24h`).
- **WebSocket:**
  - Adres `wss://stream.bybit.com/v5/public/{spot|linear}`.
  - Tüm piyasa için tek bir fiyat kanalı yok.
  - 26 Kasım 2026'dan itibaren bağlantı başına en fazla 100 abonelik.
  - İstemci 20 saniyede bir `{"op":"ping"}` gönderir.
- **Bölge:** ABD ve Çin IP'leri 403 alır. Türkiye için ayrı adresler var: `api.bybit.tr`, `stream.bybit.tr`.
