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

**Tıklama gerektiren ekranları görmek:**
- Tıklanamadığı için `Fitil::new()` içine geçici satırlar eklenir. Örneğin indikatör eklenir ya da `app.dialog = Some(...)` ile bir pencere açılır.
- Bu satırların sonuna `// TEMP-VERIFY` yazılır. Görüntü alındıktan sonra silinir, `grep -rn TEMP-VERIFY crates/` boş dönmeli.
- Mantık için ayrıca `main.rs` testlerinde `app()` yardımcısı var: uygulamayı bellek içi önbellek ve kullanıcı verisiyle kurar. Testler `update(Message::…)` çağırıp durumu kontrol eder, `view()` ile ekranların hatasız kurulduğunu doğrular.

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
| `fitil-core` | Alan modeli | `Candle` (`quote_volume`, `trades`, `taker_buy_volume` alanları `Option`), `ExchangeId`, `MarketKind` (Spot, LinearPerpetual), `Market` (= borsa + tür), `Symbol`, `Ticker`, `SeriesKey`, `Timeframe`, `aggregate`, `aggregate_whole`, çizim modeli (`drawing.rs`: `Drawing`, `DrawingKind`, `Anchor`, `LineStyle`, `Rgb`, `FIB_LEVELS`) |
| `fitil-exchange` | Borsa arayüzü ve ortak araçlar | `Exchange` trait (async-trait), `CandleQuery` (Latest/From/Until), `Features`, `RequestUsage`, `ExchangeError` (Clone), feed tipleri (`Topic`, `FeedCommand`, `FeedEvent`), `stream::run_stream` ve `StreamProtocol`, `RateLimiter` (pencere süresi ayarlanır), `Clock`, `http_client()` |
| `fitil-binance` | Binance adaptörü | `rest.rs` (piyasa başına REST istemcisi, istek ağırlığı), `stream.rs` (`BinanceStream` protokolü), `wire.rs` (JSON). Kayıtlı yanıtlar `tests/fixtures/binance/`'te. |
| `fitil-store` | SQLite, iki dosya | `Store` = `cache.sqlite` (göç 1 ilk şema, göç 2 anahtara `exchange`, isteğe bağlı alanlar NULL, `usdm` → `perp`). `UserData` = `user.sqlite` (göç 1: `indicator_templates`, göç 2: `drawings`, sembol başına bir JSON). Göç kodu ortak: `migrate(conn, MIGRATIONS)`. |
| `fitil-market` | Uygulama katmanı | `loader.rs` (`open_series`, `load_older`, `save_candle`), `series.rs` (`Series`: bellekteki mumlar, canlı mum, `LiveUpdate`), `board.rs` (`Board`: semboller ve 24 saatlik verileri; `search`, `search_markets`, `default_symbol`, `load_market`), `studies.rs` (`Studies`, `StudyId`, `is_available`), `drawings.rs` (`load_drawings`, `save_drawings`), `templates.rs` (ortak `with_user`, `Template`, `load/save/delete_template`, `SharedUserData`), `fake.rs` (testler için sahte borsa) |
| `fitil-indicators` | İndikatör hesapları | `primitives.rs` (Sma, Ema, Wma, Deviation…), `indicators/` (22 indikatör), `catalog.rs` (`Definition`, `catalog()`, `definition(id)`), `settings.rs` (`Settings`, `Params`, `Rgb`, `PALETTE`), `calculation.rs` (`Calculation`: artımlı hesap) |
| `fitil-chart` | Grafik bileşeni | iced Canvas üzerinde, sıfırdan yazıldı. `chart.rs` (durum, panel ağırlıkları), `draw.rs` (çizim, yerleşim, fare), `scale.rs`, `study.rs` (grafiğin indikatör modeli), `drawings.rs` (çizim düzenleme, geri al/yinele, zaman ↔ çubuk), `shapes.rs` (çizimlerin ekrandaki hâli, tıklama bulma, mıknatıs) |
| `fitil-app` | Pencere ve ekranlar | `main.rs` (durum, mesajlar, `exchanges()` borsa listesi, `--open`), `view.rs` (araç çubuğu, grafik alanı, durum çubuğu), `dialogs.rs` (sembol arama, indikatörler, ayarlar), `studies.rs` (indikatör → grafik modeli, etiketler), `icons.rs` (Lucide simgeleri), `feed.rs` (canlı veri aboneliği), `storage.rs` (`cache.sqlite` yerel, `user.sqlite` dolaşan klasörde), `error.rs`, `format.rs`, `i18n.rs` |
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
- **Canvas'ta yazılar şekillerin üstünde:** Aynı katmandaki bütün yazılar bütün şekillerden sonra çiziliyor. Önbellekli katmandaki bir yazı, üst katmandaki bir kutunun içinden görünür. Bu yüzden eksen yazıları her karede çizilen katmanda, kutuların altına düşenler atlanıyor.
- **`with_clip` koordinatları kaydırmaz** (0.14): kırpılmış alanda da mutlak koordinatlarla çizilir.
- **`Subscription::map` ve `filter_map`** sadece yakalama yapmayan (boyutu sıfır) kapanışları kabul eder.
- **Klavye:**
  - `iced::event::listen_with(fn)` her olayı durumuyla (`Captured` ya da `Ignored`) verir. Yazı kutusu Esc'i yakalasa bile pencereyi kapatabilmek için bu kullanılıyor (`main.rs` içinde `key_message`).
  - `keyboard::listen()` ise sadece yakalanmamış tuşları verir.
  - Yazı kutusu yukarı ve aşağı okları yakalamaz.
- **Kaydırma:** `operation::snap_to(id, RelativeOffset { x: None, y: Some(oran) })`. Bunun için `scrollable(…).id("kimlik")` verilmiş olmalı.
- **Odak:** `text_input(...).id("kimlik")` ve `iced::widget::operation::focus("kimlik")` bir `Task` döndürür.
- **Pencere (modal):** `stack![taban, opaque(mouse_area(center(opaque(içerik)).style(…)).on_press(kapat))]`.
- **Kaydırma çubuğu:** `scrollable(…).spacing(6)` çubuğu içeriğin üstüne değil yanına koyar.
- **Diğer yardımcılar:** `row(…).wrap()` satırı sığmayınca alta geçirir. Ayrıca `tooltip(içerik, ipucu, tooltip::Position::Bottom)`, `checkbox(değer).label(…).on_toggle(…)` ve `button(…).on_press_maybe(Option<Message>)` kullanılıyor. `Option<eleman>` doğrudan `push` edilebilir (`None` boş yer kaplar).
- **Özel widget (`advanced` özelliği açık):** `iced::advanced::{Widget, overlay, layout, …}`. Örnek `fitil-app/src/flyout.rs`: bir düğme ve yanında açılan menü. Menü bir `overlay` olarak her şeyin üstünde çizilir.
  - Örnek alınacak kaynak: iced'in `tooltip.rs` (konumlanan overlay) ve `float.rs` (olayları içeriğe ileten overlay).
  - Overlay'in `mouse_interaction`'ı `None` dışında bir şey döndürünce alttaki widget'lar imleci görmez. Olayı `shell.capture_event()` ile yakalayınca alttakilere hiç gitmez.
  - Overlay'in `layout`'u bütün pencerenin boyutunu alır; menüyü pencereye sığdırmak orada yapılır.
- **Kesikli çizgi hatası (iced 0.14):** `Stroke` ile `LineDash` verilen bir yolda iced, ayrı parçaları tek çizgi gibi dolaşır. Bir kesik bir parçanın sonundan ötekinin başına geçince araya düz bir çizgi çekilir. Ayrıca kesik uzunluğundan kısa bir yol hiç çizilmez. Bu yüzden `shapes.rs` uç uca eklenen çizgileri tek yol, ayrı çizgileri ayrı yol yapar (`runs`).
- **Canvas, düğme içinde:** `canvas::Canvas::new(program).width(…).height(…)` bir düğmenin içine konabilir. Tıklamayı düğme alır. Renk temadan `draw` içinde okunur (`theme.extended_palette()`).

## Grafik motoru (`fitil-chart`)

- **Durum:** `Chart` kendi canvas'ının programıdır (`impl canvas::Program<ChartMessage> for Chart`). Uygulama `chart.view().map(Message::Chart)` ile gösterir, gelen mesajı `chart.update(msg)` ile uygular.
- **Görünüm:** Görünüm çubuk sırası üzerinden tutulur. Zaman ekseni çubuk sırasıyla ilerler: boşluklar veya ay uzunlukları sorun olmaz.
- **Eski mumlar:** Eski mumlar `set_candles` ile tüm dizi verilerek eklenir. Grafik, eski ilk mumun yeni dizideki yerini bulup görünümü kaydırır.
- **Etiketler:** Grafiğin dil metinleri (`ChartLabels`) uygulamadan, Fluent'ten gelir. `chart-months` ve `chart-magnitudes` anahtarları virgülle ayrılmış listelerdir.
- **Renkler:** Yükselen #2EBD85, düşen #F6465D. Şimdilik sabit, Adım 7'de ayarlanabilir olacak.
- **İndikatörler:** Grafik `Study` listesi alır (`set_studies`), değerler mumlarla aynı sırada ve uzunlukta olur. `Plot.offset` çizgiyi çubuk sayısı kadar kaydırır (Ichimoku). Gizli indikatörler grafiğe hiç verilmez.
- **Paneller:** Yükseklik ağırlıkla tutulur: fiyat paneli 3, her yeni panel 1. `ChartMessage::MoveDivider` kenar sürükleyince ağırlık aktarır, panel 30 pikselden küçülmez. Ağırlıklar indikatör kimliğiyle saklanır, gizlenip açılan panel yüksekliğini korur.
- **Sembol değişince** `replace_series` kullanılır: grafik nesnesi kalır, mumlar ve konum sıfırlanır, yakınlaştırma ve paneller korunur.

## Çizimler

- **Veri:** `fitil_core::Drawing`. Noktalar zaman ve fiyattır (`Anchor`), ekran noktası değil. Grafik `bar_of` / `time_of` ile çubuk konumuna çevirir: mum aralarında oranla, mumların dışında zaman dilimi hızıyla. Böylece çizimler her zaman diliminde doğru yerde durur.
- **Akış:**
  - Canvas fare olaylarını `ChartMessage`'a çevirir: `PlacePoint`, `PreviewPoint`, `SelectDrawing`, `MoveDrawing`, `FinishMove`, `Cancel`.
  - Uygulama bunları `chart.update` ile uygular, sonra `take_changed_drawings` ile değişeni alıp kaydeder.
  - Ara sürükleme adımları kaydedilmez, bırakınca (`FinishMove`) kaydedilir.
- **Kayıt sırası:** `DrawingSaves` (`main.rs`) kayıtları teker teker yapar. Biri sürerken gelen aynı sembolün kaydı bekleyenle birleşir. Açılan sembollerin çizimleri `known`'da tutulur, geri dönünce okumadan gösterilir. Geç gelen okuma (`DrawingsLoaded`) yeni değişikliklerin üstüne yazmaz.
- **Çizim:** Çizimler önbellekli katmana değil, her karede çizilen katmana çizilir. Bu yüzden sürüklerken mumlar yeniden çizilmez. Fiyat ekseni yazıları da bu katmanda; yatay çizgilerin kutuları için yer açılır. Dikey çizgilerin ve kesişim çizgisinin tarihi zaman ekseninde (`line_times`).
- **Tutamaçlar:** `Shape.handles[i]` her zaman `points[i]`'yi sürükler. Bir şekil tutamacı noktanın kendisinden başka yere koyabilir: düz tepe/dipte 3. nokta düz çizginin ucunda, regresyonda noktalar regresyon çizgisinin üstünde durur.
- **Geometri (ekran koordinatlarında):** Kanallar ve dirgenler ekran noktalarıyla hesaplanır, böylece log ölçekte de düz görünür. Dirgenlerin orta çizgisi `pitchfork_frame`'de (dışarı açık; `icons.rs` de simgeleri bununla çiziyor). Regresyon ise fiyatlarla hesaplanır (kapanışlar, en küçük kareler, ±2 standart sapma, sapma `n-1`'e bölünür).
- **Yeni çizim türü eklemek:**
  1. `DrawingKind`'e ekle (`ALL`, `points`, `id`).
  2. `shapes.rs` içindeki `shape` fonksiyonuna nasıl görüneceğini yaz. Tıklama bulma aynı şekli kullanır.
  3. Bir gruptaysa `fitil-app/src/tools.rs`'te bölümüne ekle ve `icons.rs`'teki `sketch` fonksiyonuna 18×18'lik resmini çiz. Grupsuz araçlar `icons::drawn` içinde bir Lucide simgesi alır.
  4. `drawing-<id>` çevirisini iki dile de ekle. Eksikse test yakalar.
- **Araç grupları (`tools.rs`):** `ToolGroup` (şimdilik sadece `Lines`), bölümleri (`Section`: başlık anahtarı ve araçlar), Alt kısayolları (`shortcut`, `by_shortcut`). Grubun düğmesi en son seçilen aracı gösterir (`Fitil::group_tools`), açık menü `Fitil::tool_menu`.

## Simgeler (Lucide)

- **Yazı tipi:** `assets/fonts/lucide.ttf` (lucide-static 1.66.0), lisansı `assets/fonts/LICENSE-lucide.txt`.
  - `main.rs` içinde `.font(icons::FONT_BYTES)` ile yüklenir.
  - `Font::with_name("lucide")` ile kullanılır.
- **Yeni simge eklemek:**
  - Simgenin kod noktası `codepoints.json`'dan bulunur. Dosya: https://cdn.jsdelivr.net/npm/lucide-static@1.66.0/font/codepoints.json
  - Kod noktası `icons.rs`'e sabit olarak eklenir.
  - Lucide sürümü değişirse kod noktaları da değişebilir. Yazı tipi ve sabitler birlikte güncellenmeli.
- **Çizgi araçlarının simgeleri Lucide'den değil:** Lucide'de çoğunun karşılığı yok. `icons.rs`'teki `ToolIcon` canvas'ı bunları TradingView'daki gibi ince çizgiler ve noktalarda küçük boş dairelerle çizer (`sketch`). Çizgiler dairelerin içinden geçmez (`outside_circles`). Grup düğmesinin sağ alt köşesindeki küçük üçgen, arkasında menü olduğunu gösterir.


## İndikatörler (`fitil-indicators`)

- **Kurallar:**
  - EMA ve Wilder (RMA) ilk değerini ilk `n` değerin SMA'sından alır (TA-Lib gibi).
  - Standart sapma popülasyon formülüyle hesaplanır (TradingView `ta.stdev` gibi).
  - SuperTrend, Parabolic SAR, DMI (`fixnan` dahil) ve Keltner (EMA + ATR(10)) TradingView'ın mantığıyla yazıldı.
  - VWAP ve CVD gün/hafta/ay başında sıfırlanır (`Timeframe::bucket_start`, UTC).
- **Çıktı sıraları katalogdadır.** Örnek: Bollinger [orta, üst, alt], MACD [histogram, macd, sinyal], Ichimoku [dönüşüm, taban, gecikmeli, öncü A, öncü B].
- **Yeni indikatör eklemek:**
  1. `indicators/` altına hesabını yaz (`Indicator` trait'i, `Clone`).
  2. `catalog.rs` listesine tanımını ekle.
  3. `locales/*/fitil.ftl` dosyalarına `indicator-<id>`, `indicator-<id>-short` ve yeni parametre ve çıktı adlarını ekle.

  Artımlı hesap testi her katalog indikatörünü kendiliğinden kapsar. Çeviri testi eksik anahtarı yakalar.
- **Artımlı hesap:** `Calculation` son kesinleşmiş mumu (tamamını) ve ilk mumun zamanını hatırlar. İkisinden biri değişirse baştan hesaplar. En yeni mum her seferinde durumun bir kopyası üzerinde hesaplanır.
- **Yuvarlama gürültüsü:** Kayan toplam yüzünden 0–100 arası bir değer 1e-14 kadar taşabilir. Testler bunu tolere eder. Grafik "-0.00" yazmaz (`without_negative_zero`).

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
