# Günlük

Yapılan her şey tarih sırasıyla burada. En yeni kayıt en altta.

---

## 30 Ekim 2026: Oturum 1

### Konuşma (kodlamadan önce)

1. **Başlangıç:** Proje sahibi TradingView'ın kısıtlarından bıktığını söyledi. Binance gibi borsaların herkese açık API'sini kullanan, herkesin indirip çalıştırabileceği bir masaüstü uygulaması istedi. Kodlamadan önce her şeyin ayrıntılı konuşulmasını istedi.
2. **İlk öneri:** Claude Electron, TypeScript, React ve CCXT ile bir mimari önerdi.
   - Temel fikir: sunucu yok, her kullanıcı borsaya doğrudan bağlanır.
   - Grafik kütüphanesi seçenekleri anlatıldı: Lightweight Charts, KLineChart, kendi motorumuz.
   - Zorluklar anlatıldı: geçmiş veri, bağlantı kopmaları, istek limitleri, sadece kripto verisi olması.
   - Fazlar, dürüst uyarılar ve sorular paylaşıldı.
3. **Rust kararı:** Proje sahibi "her şeyi Rust ile yapalım, daha hafif ve hızlı olmaz mı" diye sordu.
   - Claude farkın nerede gerçekten hissedileceğini anlattı: çok sayıda grafik, düşük RAM, ısı haritası ve footprint gibi ağır grafikler.
   - Tauri ile tam native Rust karşılaştırıldı. Tauri'de grafik yine JavaScript ile çizileceği için **tam native Rust ve iced** önerildi.
   - Bu yolun yapılabilir olduğuna örnek olarak açık kaynak Flowsurface projesi gösterildi.
4. **Proje sahibinin kararları:**
   - Al-sat olmayacak. Çizim ve indikatör olacak, backtest sonra gelecek.
   - Açık kaynak olacak, GitHub'dan indirilecek.
   - Arayüz Türkçe ve İngilizce olacak, sonra yeni diller eklenebilecek.
   - Windows, macOS ve Linux desteklenecek.
   - Kodun tamamını Claude yazacak, proje sahibi Rust bilmiyor.
5. **Lisans:** GPL-3.0 seçildi.
6. **İsim ve borsa:** Fitil seçildi. Önce Binance. GitHub hesabı `ylcnnemre`, `gh` ile bağlı.
7. **Tasarım belgesi:** `docs/TASARIM.md` yazıldı. İçinde ilkeler, kapsam dışı konular, Faz 1'in ayrıntılı kapsamı, sonraki fazlar, mimari, test, dağıtım, 7 adımlık yol haritası ve riskler var.
8. **Eksikler:** Proje sahibi "başka ne unutmuş olabiliriz" diye sordu. Claude 17 eksik konu çıkardı, proje sahibi hepsinin eklenmesini istedi. Belgeye eklenenler:
   - Grafik görüntüsü alma, indikatör şablonları, alış/satış hacmi ve CVD, yedekleme.
   - Ekran ölçeği ve renk körü modu.
   - Telemetri olmaması.
   - Güncellemede veri kaybı olmaması, önbellek yönetimi, arka planda az kaynak kullanımı.
   - Saat düzeltmesi, proxy desteği.
   - Yasal uyarı.
   - Sahte kopyalara karşı önlemler, katkı inceleme süreci, lisans kontrolü.
   - İkon ve topluluk dosyaları.
9. **Onay ve hafıza:** Proje sahibi tasarım belgesini ve 1. adımı onayladı. Ayrıca bu `memory/` klasörünün oluşturulmasını istedi, yeni oturumlarda hiçbir şey unutulmasın diye.

### Yapılanlar

- `docs/TASARIM.md` oluşturuldu ve 17 ek konuyla güncellendi.
- `memory/` klasörü ve proje kökündeki `CLAUDE.md` oluşturuldu. `CLAUDE.md`, yeni oturumlarda `memory/01` ve `memory/02` dosyalarını otomatik yükler.
### Adım 1: Geliştirme ortamı ve proje iskeleti (tamamlandı)

**Kurulumlar** (proje sahibinin onayıyla, winget ile):
- Visual Studio Build Tools 2022, VCTools iş yükü.
- Rustup ve Rust 1.99.1 (stable, MSVC).
- `cargo install cargo-deny --locked` ile cargo-deny 0.20.2.

**Proje iskeleti:**
- Cargo workspace (edition 2024) ve 6 crate:
  - `fitil-app`: binary adı `fitil`.
  - `fitil-core`, `fitil-exchange`, `fitil-store`, `fitil-indicators`, `fitil-chart`: şimdilik sadece açıklama satırı var.
- `fitil-app` içeriği:
  - iced 0.14 ile 1280×800 boyutunda, ortalanmış, koyu temalı bir pencere.
  - Ekranda başlık, alt başlık, durum metni, sürüm ve Türkçe/English düğmeleri var.
  - `i18n.rs`: Fluent dosyalarını derleme anında gömüyor, sistem dilini `sys-locale` ile seçiyor, Unicode izolasyon işaretlerini kapatıyor.
  - 4 test var. Biri, tüm dillerin aynı anahtarları tanımladığını kontrol ediyor.
- `locales/tr/fitil.ftl`, `locales/en/fitil.ftl`.

**Depo dosyaları:**
- LICENSE: GPL-3.0, GitHub'ın resmi metni.
- README (Türkçe ve İngilizce), CONTRIBUTING (İngilizce, başında Türkçe özet, çeviri rehberi dahil), CHANGELOG.
- Issue şablonları (hata, özellik).
- `assets/icon.svg`: ikon taslağı. Yeşil bir mum, üst fitilinde küçük bir alev.
- `.gitignore`, `.gitattributes` (LF), `rust-toolchain.toml` (stable, rustfmt ve clippy), `deny.toml` (GPL uyumlu lisans listesi).
- CI (`.github/workflows/ci.yml`):
  - fmt ve clippy (Linux),
  - derleme ve test (Windows, macOS, Linux),
  - cargo-deny.
- `scripts/capture-window.ps1`: çalışan Fitil penceresinin ekran görüntüsünü alan yardımcı betik. Claude arayüzü bununla kontrol eder.

**Doğrulama:**
- Yerelde `cargo build`, `test`, `fmt`, `clippy` ve `cargo deny check` temiz geçti.
- Uygulama açıldı. Ekran görüntüsünde Türkçe otomatik seçilmiş, Türkçe karakterler düzgün çiziliyor.
- Debug derlemede bellek kullanımı yaklaşık 217 MB. Başarı ölçütleri release derlemeyle ölçülmeli.

**GitHub:**
- `gh repo create ylcnnemre/fitil --public` ile depo açıldı.
- İlk commit `3c480ef` gönderildi. İlk CI çalışması 5 işin hepsinde yeşil.

**Son olarak:** Proje sahibi "bir sonraki adımda ne yapıyoruz" diye sordu. Adım 2'nin planı anlatıldı ve proje sahibi "Evet, başla" dedi.

### Adım 2: Binance bağlantısı ve yerel depo (tamamlandı)

**Önce doğrulananlar** (Binance belgeleri ve canlı istekler):
- **Vadeli WebSocket adresi değişti:** 23 Nisan 2026'dan beri eski `wss://fstream.binance.com/stream` çalışmıyor. Mum ve mini-ticker akışları artık `wss://fstream.binance.com/market/stream` adresinden geliyor.
- **Spot `!ticker@arr` kaldırıldı:** 26 Mart 2026'da emekliye ayrıldı. Yerine `!miniTicker@arr` kullanılıyor.
- **Sadece `endTime` ile mum isteği:** O zamana kadarki son `limit` mumu döndürüyor. Listelemeden öncesi istenirse daha az mum geliyor, bu da geçmişin başına ulaşıldığını gösteriyor.
- **Binance'in 3d mumları** epoch'a göre 1 gün kaymalı hizalı. Bu yüzden 3d, özel zaman dilimlerine kaynak yapılmıyor.
- **Mum akışı sadece işlem olunca güncelleme gönderiyor.** USDTIDR gibi az işlem gören paritelerde saniyelerce mesaj gelmeyebiliyor.
- **Vadelide yeni sözleşme tipi:** `TRADIFI_PERPETUAL` (örneğin XAUUSDT). Perpetual olarak dahil edildi.

**Yazılanlar:**
- **`fitil-core`:**
  - `Candle` (taker alış hacmi dahil), `Market` (Spot, UsdMFutures), `Symbol`, `Ticker`, `SeriesKey`.
  - `Timeframe`: ayrıştırma, kovaya hizalama, `best_source` ve `aggregate`.
  - 11 test.
- **`fitil-store`:**
  - SQLite, `user_version` ile şema göçü (migration), WAL modu.
  - Mum kaydetme ve okuma, aralık, geçmiş başlangıcı ve seri silme işlemleri.
  - 7 test.
- **`fitil-exchange`:**
  - `BinanceClient`: sembol listesi, 24 saatlik ticker, mumlar, saat senkronu, istek ağırlığı takibi, 429/418/451 hataları.
  - `run_feed`: WebSocket, abone olma ve abonelikten çıkma, 30 saniye sessizlikte bağlantıyı ölü sayma, 1-2-4-8-16-30 saniye aralıklarla yeniden bağlanma.
  - TLS için rustls ve ring kullanılıyor.
  - Testler:
    - 15 birim testi, kaydedilmiş gerçek yanıtlarla (`tests/fixtures/binance/`).
    - 2 canlı test (`--ignored`).
- **`fitil-app`:**
  - `data.rs`:
    - önbellek önce, eksikler Binance'ten,
    - 50 istekten büyük boşlukta seri sıfırdan indirilir,
    - açık mum ayrıca çekilir.
  - `view.rs`: üç bölümlü ekran.
  - `format.rs`: dile göre sayı biçimi.
  - `error.rs`: hatalar Fluent anahtarlarına bağlanır.
  - Bir canlı test, veri katmanını uçtan uca doğruluyor.
- **Diğer:**
  - Önbellek konumu: Windows'ta `%LOCALAPPDATA%\Fitil\cache.sqlite`. `docs/TASARIM.md` §6.3 buna göre güncellendi (o bölüm sonradan §6.6 oldu).
  - `deny.toml`: `CDLA-Permissive-2.0` eklendi (HTTPS kök sertifika listesinin veri lisansı), `allow-wildcard-paths` açıldı.

**Ekran denemesinden çıkan düzeltmeler:**
- **Sıralama:** Sembol listesi farklı para birimlerindeki hacimleri doğrudan karşılaştırdığı için USDTIDR en üstte çıkıyordu. Hacimler artık dolara çevrilip sıralanıyor.
- **Hizalama:** Liste altındaki sayım metni kenara yapışıktı, düzeltildi.
- **Açık mum:** Az işlem gören paritelerde açık mum hiç görünmüyordu. Sembol açılırken açık mum REST ile de çekiliyor.

**Test araçları:**
- `scripts/capture-window.ps1` artık PrintWindow kullanıyor: pencere arkadayken de çalışıyor ve kullanıcının odağını çalmıyor.
- PostMessage ile tıklama denendi ama iced bu tıklamaları almadı. Betik silindi.
- Arayüzdeki tıklama akışlarını proje sahibi deniyor. İlk denemesinde USDTIDR seçme ve eski mum yükleme çalıştı.

**Doğrulama:**
- Toplam 41 test geçiyor.
- 3 canlı test geçti: REST, WebSocket (vadelinin yeni adresi dahil) ve veri katmanı.
- fmt, clippy ve cargo-deny temiz.
- Debug derlemede bellek yaklaşık 206 MB.

---

## 31 Ekim 2026: Oturum 1 (devam)

### Adım 3: Grafik motoru (tamamlandı)

**Yapılanlar:**
- **Yeni crate `fitil-chart`:** iced Canvas üzerinde, sıfırdan yazıldı.
  - `chart.rs`: Grafik durumu.
    - Görünüm çubuk sırası (index) üzerinden tutuluyor: `right` (sağ kenardaki çubuk) ve `spacing` (çubuk başına piksel).
    - Eski mumlar başa eklenince görünüm aynı yerde kalıyor. En yeni mum görünüyorsa yeni mumlar takip ediliyor.
    - Fiyat aralığı otomatik ya da elle ayarlanıyor.
    - Heikin Ashi dönüşümü burada.
  - `scale.rs`:
    - Log ve yüzde dönüşümleri, "yuvarlak" fiyat çizgileri (1, 2, 2,5 ve 5 katları).
    - Yerel saate göre zaman etiketleri: saat, gün, ay adı, yıl.
    - Geri sayım ve hacim kısaltmaları.
  - `draw.rs`:
    - İki katman çiziliyor. Önbellekli katman ızgara, hacim, mumlar ve eksenlerden oluşuyor. Her karede yeniden çizilen katman son fiyat, artı imleci ve göstergeden (lejant) oluşuyor.
    - Fare olayları `ChartMessage` mesajlarına çevriliyor: Pan, Zoom, ScalePrice, ResetPrice, ResetView.
    - Dikey sürükleme 12 pikseli geçince fiyat ölçeği elle ayarlanmış sayılıyor.
  - Görünüm durumu uygulamada tutuluyor. Böylece Adım 6'da kaydedilebilecek ve grafikler arasında eşlenebilecek.
- **`fitil-core`:** `aggregate_whole` eklendi. Özel zaman dilimlerinde baştaki eksik kova atlanıyor.
- **`fitil-app`:**
  - `data.rs`:
    - `open_series` en yeni pencereyi döndürüyor (görüntülenen zaman diliminden 1000 mum kadar süre).
    - `load_older(before)` önce önbelleğe, yoksa Binance'e gidiyor.
  - Yeniden bağlanınca bellekteki eski mumlar korunarak yeniler ekleniyor (`merge_newest`).
  - "Daha eski mumları yükle" düğmesi kaldırıldı. Grafik sola kaydırılınca yükleme kendiliğinden başlıyor (`Chart::wants_older`).
  - Eski metin tabanlı ayrıntı paneli kaldırıldı. Yerine araç çubuğu, grafik ve kısa kullanım ipucu geldi. Fiyat listesi 400 piksel sabit genişlikte.
  - `--open market:SYMBOL:tf` komut satırı seçeneği eklendi. Claude ekrana tıklayamadığı için bununla test ediyor.
- **Testler:** 56 test (`fitil-chart` 14, `fitil-core` 12, `fitil-exchange` 15, `fitil-store` 7, `fitil-app` 8) ve 3 canlı test.

**Ekran denetiminde bulunanlar:**
- **Kırpma hatası:** Fiyat ölçeği elle ayarlanınca mumlar canvas'ın dışına, araç çubuğunun üstüne taşıyordu. Mumlar ve hacim artık `frame.with_clip(plot)` içinde çiziliyor.
- **Sütun aralığı:** Fiyat listesindeki sütunlar sıkışıktı, aralık eklendi.

**Ekranda doğrulananlar** (BTCUSDT 1h ve ETHUSDT Vadeli 45m):
- mumlar ve hacim,
- eksen etiketleri (gün numaraları),
- son fiyat kutusu ve geri sayım,
- artı imleci ve zaman etiketi ("29 Eki 2026 01:30"),
- özel zaman diliminin 15m'den üretilmesi,
- uzaklaştırınca eski sayfaların kendiliğinden yüklenmesi.

Bu denemeleri proje sahibi uygulamayı kullanırken yaptı.

**Proje sahibinin sorusu:** "Grafik ekranını biz mi yaptık, bir yerden mi aldık?"
- Cevap: Grafik tamamen bizim, `fitil-chart` crate'inde sıfırdan yazıldı.
- Dışarıdan sadece genel amaçlı kütüphaneler kullanıldı: iced (pencere ve çizim komutları), chrono (yerel saat).
- TradingView'ın kütüphanesi ya da başka bir grafik kütüphanesi kullanılmadı. Flowsurface'tan kod alınmadı.
- 1-2-5 fiyat çizgileri ve Heikin Ashi formülü herkesin kullandığı bilinen yöntemler.

**Not:** Proje sahibi bir ara oturumu durdurdu ("durdurmak zorunda kaldım") ve "devam edelim" dedi. O sırada uygulamayı kullanıyordu. Claude yeniden başlatırken proje sahibinin açık penceresini kapatmış oldu. Bundan sonra kullanıcı uygulamayı kullanıyorsa süreç kapatılmamalı.

### Mimari düzenleme: çok borsalı, port/adaptör (tamamlandı)

**Nasıl başladı:**
- Proje sahibi sordu: "Projede kullandığımız mimari ne? Mimariyi pek konuşmadık ama temiz ve düzgün bir mimari kullanmak daha iyi olabilir."
- Claude mevcut durumu anlattı. Katmanlar ayrıydı ama iki eksik vardı: iş mantığı arayüze karışmıştı (`main.rs` 734 satır) ve borsalar için ortak bir arayüz yoktu.
- Claude port/adaptör düzenini önerdi.
- Proje sahibi şunu ekledi: "Projeye Hyperliquid, OKX, Huobi gibi başka borsalar eklenebilir. Ayrıca farklı modüller de gelebilir, hepsini düşünerek hareket etmemiz daha iyi olur."

**Yapılanlar:**
1. **Tasarım belgesi:** `docs/TASARIM.md` §6 yeniden yazıldı:
   - katmanlar ve bağımlılık tablosu, 6 kural,
   - veri modeli (borsa, piyasa, sembol, mum),
   - `Exchange` arayüzü, yeni borsa ekleme adımları, klasör yapısı.

   Faz 2'deki borsa listesine Hyperliquid ve HTX eklendi.
2. **Çekirdek:**
   - Yeni tipler: `ExchangeId`, `MarketKind` (Spot, LinearPerpetual), `Market { exchange, kind }`.
   - `Candle` içindeki `quote_volume`, `trades` ve `taker_buy_volume` artık `Option`. Birleştirmede biri bilinmiyorsa toplam da bilinmiyor sayılır.
3. **`fitil-exchange`, borsa arayüzü:**
   - `Exchange` trait'i.
   - Ortak WebSocket motoru (`run_stream` ve `StreamProtocol`): ping yanıtı, düzenli ping gönderme, sessizlik süresi, yeniden bağlanma.
   - `RateLimiter` artık her pencere süresiyle çalışıyor ve bildirmeyen borsalar için yerel sayaç tutuyor.
   - `Clock`, `http_client`.
   - Testler: yerel bir WebSocket sunucusuyla internetsiz yeniden bağlanma testi dahil, 9 test.
4. **`fitil-binance`:** Binance kodu ayrı bir crate'e taşındı (git geçmişi korundu) ve `Exchange` arayüzünü uyguluyor. 9 test ve 2 canlı test.
5. **`fitil-store`, göç 2:** Anahtara `exchange` eklendi, isteğe bağlı alanlar NULL olabiliyor, `usdm` → `perp`. 10 test var, ilk sürümün önbelleğini yükseltme testi dahil.
6. **`fitil-market`, uygulama katmanı:**
   - `loader.rs`: eskiden `fitil-app/data.rs` idi.
   - `series.rs`: canlı mum, boşluk algılama, birleştirme ve eski mum ekleme kararları burada.
   - `board.rs`: fiyat listesi ve dolar hacmine göre sıralama.
   - Sahte borsayla internetsiz 13 test.
7. **`fitil-app`:**
   - Sadece ekran ve bağlantılar kaldı.
   - Yeni dosyalar: `exchanges()` borsa listesi, `feed.rs`, `storage.rs`.
   - Üst çubukta borsa seçici var.
   - Hata metinleri borsa adı içermiyor ("Borsaya ulaşılamadı").
   - `--open` artık `[borsa:]piyasa:SEMBOL[:tf]` biçiminde.
   - `FITIL_DATA_DIR` ile önbellek klasörü değiştirilebiliyor.
   - Uçtan uca canlı test `tests/live.rs`'e taşındı.

**Doğrulama:**
- Toplam 77 test ve 3 canlı test (Binance REST, WebSocket, uçtan uca yükleme) geçti. fmt, clippy ve cargo-deny temiz.
- Ekranda: proje sahibinin gerçek önbelleğinin kopyası (`FITIL_DATA_DIR`) yeni sürümle açıldı.
  - ETHUSDT vadeli 15m mumların hepsi taşındı: 2.999 mum ve aradaki 2 yeni mum.
  - Grafik, borsa seçici ve durum çubuğu doğru göründü.
- Proje sahibinin açık penceresi (eski sürüm) kapatılmadı. Testler `target/verify` klasöründe derlendi.

**Bilinen küçük sorun:** Önbellek yolu çok uzunsa durum çubuğu iki satıra bölünüyor. Sadece test klasöründe görüldü.

**Diğer borsalar için araştırma:** OKX, Hyperliquid, HTX ve Bybit API'leri arka planda araştırıldı. Sonuçlar `04-teknik-notlar.md` dosyasında "Diğer borsalar" başlığında.

### Adım 4: İndikatörler ve paneller (kodlandı, proje sahibinin denemesi bekleniyor)

Proje sahibi "Evet, başla" dedi. Oturum uzun sürdüğü için arada bir kez özetlendi (context sıkıştırma). İş kaldığı yerden devam etti.

**Yapılanlar:**
1. **`fitil-indicators`:**
   - `primitives.rs`: ortak yapı taşları: `Window`, `Sma`, `Ema` (normal ve Wilder/RMA, ikisi de SMA ile başlar), `Wma`, `Deviation` (popülasyon), `MeanDeviation`, `Extremes`, `TrueRange`, `Atr`, `Change`. NaN girdiyi atlar.
   - `indicators/`: 22 indikatör, TradingView'ın tanımlarıyla (SuperTrend, SAR, DMI, Keltner gibi).
   - `catalog.rs`: her indikatörün kimliği, grubu, yeri (fiyatın üstü ya da panel), parametreleri, çıktıları, dolgu (iki çizgi arası), seviye çizgileri, sabit aralık, sayı biçimi, alıcı hacmi gerekip gerekmediği.
   - `settings.rs`: kaydedilebilir ayarlar (JSON), bozuk/eski ayarları onaran `normalized()`.
   - `calculation.rs`: artımlı hesap. En yeni mum hep bir kopya üzerinde hesaplanır, önceki mumlar "kesinleşmiş" sayılır. İlk mum ya da son kesinleşmiş mumun kendisi değişirse baştan hesaplanır.
   - Testler: StockCharts'ın RSI(14) ve EMA(10) örnek tabloları (4 ondalıklı fiyatlarla), her indikatör için "canlı güncelleme = baştan hesap" eşitliği.
2. **`fitil-store`:** `UserData` (`user.sqlite`), göç 1: `indicator_templates` tablosu. Göç kodu önbellekle ortak (`migrate`).
3. **`fitil-market`:**
   - `studies.rs`: `Studies` (ekle, kaldır, ayar değiştir, gizle, güncelle). Aynı tür ikinci bir ortalama paletten boş bir renk alır. Gizli indikatör hesaplanmaz, gösterilince yetişir.
   - `templates.rs`: şablon kaydet/oku/sil. Biçim `{"version":1,"studies":[…]}`. Okunamayan tek indikatör diğerlerini bozmaz.
   - Alıcı hacmi bildirmeyen borsada Alış/Satış Hacmi ve CVD listede çıkmaz, şablondan da eklenmez (`is_available`).
4. **`fitil-chart`:**
   - `study.rs`: grafiğin kendi indikatör modeli (`Study`, `Plot`, `PlotFill`). Grafik hesap yapmaz, `fitil-indicators`'a bağımlı değildir.
   - Alt paneller: yükseklikler ağırlıkla tutulur (fiyat 3, her panel 1). Kenar sürüklenince ağırlık aktarılır, panel 30 pikselden küçülmez. Ağırlıklar indikatör kimliğiyle saklanır.
   - Çizim: çizgi (NaN'da kopar), histogram (sabit, işarete göre, muma göre renk), nokta (SAR), iki çizgi arası dolgu (kesişince renk değişir: Ichimoku bulutu), kaydırılmış çizgiler (Ichimoku), seviye çizgileri.
   - Fiyatın üstündeki indikatörler otomatik ölçeğe dahil.
   - Lejant: her indikatör bir satır, değerler kendi renginde, arkasında yarı saydam zemin.
   - Artı imleci tüm panellerden geçer. Kutu, imlecin bulunduğu panelin ekseninde çıkar.
   - `replace_series`: sembol değişince grafik nesnesi kalır, yakınlaştırma ve panel yükseklikleri korunur.
5. **`fitil-app`:**
   - `studies.rs`: uygulama katmanı → grafik modeli dönüşümü, etiketler ("RSI 14", "BB 20 2"), sayı okuma (virgül de olur), arama.
   - `dialogs.rs`: karartılmış arka planlı pencere (`stack!` + `opaque` + `mouse_area`), İndikatörler penceresi (gruplu liste, arama, grafikteki adet, şablonlar), ayar penceresi (girdiler, renk kutuları, kalınlık, göster).
   - Esc pencereyi kapatır (`keyboard::listen`). İndikatörler penceresi açılınca arama kutusuna odaklanır.
   - Şablon silme iki tıklama ister ("Emin misin?").
   - Yeni çeviri anahtarları (TR/EN). Her katalog kimliğinin çevirisinin olduğunu bir test denetler.
6. **Belgeler:** `docs/TASARIM.md` §6 tablosu (store, market, chart) ve §6.6 (`user.sqlite` + `cache.sqlite`), `CHANGELOG.md`.

**Ekran denetiminde bulunanlar ve düzeltilenler:**
- **Lejant okunmuyordu:** MACD'nin mavi çizgisi "-102.43" yazısının eksisini örtüyordu. Lejant satırlarına yarı saydam zemin eklendi.
- **Panel ekseninde sayı yoktu:** RSI panelinde iki çizgi aralığı kenarlara düşüyordu. Çizgi sıklığı artırıldı, seviye değerleri (70/50/30) de eksene yazılıyor.
- **Eksen yazıları kutuların içinden görünüyordu** (önceki adımlardan kalma hata): iced bir katmandaki bütün yazıları şekillerden sonra çiziyor. Eksen yazıları her karede çizilen katmana taşındı, kutuların altında kalanlar atlanıyor.
- **Liste sayıları kaydırma çubuğunun altında kalıyordu:** kaydırma çubuğu içeriğin yanına alındı (`scrollable(...).spacing(6)`).
- **Palette olmayan varsayılan renk** (Histogram'ın yeşil-turkuazı) seçili görünmüyordu: palette yoksa ilk kutu olarak eklendi.

**Ekranda doğrulananlar** (BTCUSDT 1h, önbellek kopyasıyla): iki EMA farklı renkte, Bollinger dolgusu, Ichimoku bulutu ve geleceğe uzanan kısmı, RSI/MACD/Hacim panelleri, lejantlar, etiket satırı, İndikatörler penceresi, MACD ayar penceresi. Doğrulama için açılışta indikatör ekleyen geçici satırlar (`TEMP-VERIFY`) kullanıldı ve sonra silindi.

**Not:** Doğrulama sırasında açılan test penceresinde (önbellek kopyası, `scratchpad`) NEARUSDT'nin seçili olduğu ve imlecin grafikte durduğu görüldü. Proje sahibi muhtemelen pencereyi denedi.

**Testler:** 132 test (`fitil-indicators` 30, `fitil-chart` 21, `fitil-market` 21, `fitil-app` 16, `fitil-store` 13, `fitil-core` 13, `fitil-exchange` 9, `fitil-binance` 9) ve 3 canlı test. fmt ve clippy temiz.

### TradingView düzeni (31 Ekim 2026, Adım 4'ün ardından)

**İstek:** Proje sahibi uygulamayı çalıştırdıktan sonra TradingView'ın ekran görüntüsünü gönderdi: "Arayüz TradingView gibi olmalı, solda coinlerin listelenmesine gerek yok, coin aratınca modal açılsın."

**Yapılanlar:**
- **Tasarım:** `docs/TASARIM.md` §4.5'e "Ana ekran" ve ayrıntılı "Sembol arama" maddeleri eklendi. İzleme listesi (Adım 6) sağda açılıp kapanan bir panel olacak. Karar 17 olarak `01-proje-ozeti.md`'ye yazıldı.
- **`fitil-market`, `board.rs`:**
  - `Board::search`: büyük/küçük harf ve ayraçlar yok sayılır (`eth/usdt` → ETHUSDT). Yakınlık sırası: tam ad, aynı coin (base), baştan eşleşme, içinde geçme. Aynı yakınlıkta dolar hacmi.
  - `search_markets` birden fazla piyasayı birleştirir.
  - `default_symbol`: en çok işlem gören BTC paritesi, yoksa en çok işlem gören sembol.
  - Eski `ranked` kaldırıldı.
- **`fitil-app`:**
  - `view.rs` baştan yazıldı. Araç çubuğu: sembol adı (tıklayınca arama), sık kullanılan zaman dilimi düğmeleri ve "Diğer" listesi, grafik tipi, Log, %, İndikatörler, Hacim, dil düğmeleri. Grafik ekranın tamamında.
  - Durum çubuğu grafiğin piyasasını gösterir. Önbellekteki mum sayısı araç çubuğundan buraya taşındı, önbellek yolu ipucunda.
  - `dialogs.rs`: sembol arama penceresi (`Dialog::Symbols`): sekmeler, yüklenemeyen piyasa için "Tekrar dene", en fazla 100 sonuç.
  - Klavye: `iced::event::listen_with(key_message)`. Esc her durumda pencereyi kapatır. Yukarı/aşağı oklar seçimi taşır. Hiçbir kutunun almadığı bir harf ya da rakam sembol aramasını açar.
  - İlk piyasa yüklenince varsayılan grafik açılır (`--open` verilmişse o açılır).
  - Kaldırılanlar: borsa seçici, Spot/Vadeli düğmeleri, üstteki arama kutusu, soldaki liste.
- **Testler:** 137 test. Yeniler: arama sıralaması, piyasaların birleşmesi, varsayılan sembol, yazınca arama açılması, oklar ve Enter, sekmeler.

**Ekranda doğrulananlar:** Test penceresi (önbellek kopyası) açıldı, BTCUSDT kendiliğinden geldi. Proje sahibi o pencerede arama penceresini açıp ETHUSDT vadeliyi seçti. Ekran görüntüleri pencerenin, sekmelerin, etiketlerin ve "2.173 sonuçtan ilk 100'ü" notunun doğru çıktığını gösterdi.

**Not:** Proje sahibinin kendi penceresi (`target\debug\fitil.exe`, eski sürüm) açık kaldı, kapatılmadı. Yeni sürüm `target\verify` klasöründe derlendi.

---

## 1 Kasım 2026

### Adım 5: Çizim araçları (kodlandı, proje sahibinin denemesi bekleniyor)

**Nasıl başladı:** Proje sahibi "sırada ne var" diye sordu. Claude Adım 5'i ve sol çubuk simgeleri için iki yolu anlattı: Lucide (indirme gerekir) ya da kendimiz çizmek. Proje sahibi "Lucide ile başla" dedi (karar 18).

**Yapılanlar:**
1. **Lucide:** `lucide-static@1.66.0` paketinden sadece `lucide.ttf` (977 KB), `codepoints.json` ve `LICENSE` indirildi (jsDelivr). Önce scratchpad'e indirilip incelendi, sonra `assets/fonts/` içine kondu. Yazı tipinin iç adı `lucide`. Lisans ISC, Feather'dan gelen simgeler MIT.
2. **`fitil-core`, `drawing.rs`:** `Anchor { time, price }`, 11 türlü `DrawingKind`, `LineStyle`, `Drawing`, `FIB_LEVELS`. `Rgb` indikatörlerden çekirdeğe taşındı. İndikatör ayarları onu `[r, g, b]` olarak kaydetmeye devam ediyor (`rgb_format`).
3. **`fitil-store`:** `user.sqlite` göç 2 ile `drawings (exchange, market, symbol, body)` tablosu. `drawings` ve `save_drawings` (`None` siler).
4. **`fitil-market`, `drawings.rs`:** JSON biçimi `{"version":1,"drawings":[…]}`. Okunamayan çizim atlanır, kalınlık 1–4'e çekilir. Ortak `with_user` yardımcısı `templates.rs`'te.
5. **`fitil-chart`:**
   - `drawings.rs`: yerleştirme (`Draft`), seçme, sürükleme (ilk hareket geri alma için kayıt alır), kilit, kopyala (5 çubuk sağa), sil, tümünü sil, geri al/yinele (100 adım), `take_changed_drawings`.
   - Zaman ↔ çubuk dönüşümü (`bar_of`, `time_of`): boşluklu mumlarda araya oranlar, mumların dışında zaman dilimi hızıyla devam eder. Aynı çizim her zaman diliminde doğru yerde durur.
   - `shapes.rs`: her çizimin ekrandaki hâli (`Shape`: çizgiler, kılavuz çizgiler, alanlar, etiketler, tutamaçlar). Aynı şekil hem çizmek hem tıklamayı bulmak için kullanılıyor. Mıknatıs `anchor_at` içinde.
   - `draw.rs`: fare olayları. Araç seçiliyken basınca nokta konur; basılı sürükleyip bırakınca ikinci nokta da konur. Çizime basınca seçilir ve sürüklenir. Çizimler her karede çizilen katmanda, yatay çizgilerin fiyatı eksende.
6. **`fitil-app`:**
   - `icons.rs` (kod noktaları, `.font(icons::FONT_BYTES)`).
   - Sol araç çubuğu, seçili çizimin satırı, geri al/yinele düğmeleri.
   - `Message::Escape`, Delete/Backspace, Ctrl+Z, Ctrl+Y ve Ctrl+Shift+Z.
   - Kayıtlar sırayla yapılıyor (`DrawingSaves`). Biri sürerken yenisi bekliyor, aynı sembolün bekleyen kaydı en yenisiyle birleşiyor. Açılan sembollerin çizimleri bellekte de tutuluyor, geri dönünce okumadan gösteriliyor.
   - Zaman dilimi değişince çizimler kalıyor, sembol değişince o sembolünkiler geliyor.

**Ekran denetiminde bulunanlar ve düzeltilenler:**
- **Sol çubuk ekranın yarısını kaplıyordu:** `scrollable` varsayılan olarak bütün genişliği istiyor. `width(Shrink)` verildi.
- **Fibonacci etiketleri kesiliyordu:** etiketler çizimin solundaydı, grafiğin kenarında kesiliyordu. Artık her seviyenin hemen üstünde, içeride.
- **Ölçümde "9s 0dk" yazıyordu:** sıfır olan ikinci birim artık atlanıyor ("9s").

**Ekranda doğrulananlar** (test penceresi, önbellek kopyası, geçici `TEMP-VERIFY` satırlarıyla 10 örnek çizim):
- dikey ve yatay çizgi, yatay çizginin eksendeki fiyatı;
- Fibonacci seviyeleri, bantları ve etiketleri;
- trend çizgisi, ışın, paralel kanal ve orta çizgisi;
- dikdörtgen, not ve ölçüm kutusu (−47,57, 9 mum);
- seçili Fibonacci'nin satırı;
- bütün Lucide simgeleri.

Geçici satırlar silindi. Proje sahibi test penceresini yine kullandı: grafik tipi listesini açtı, imleci gezdirdi.

**Testler:** 154 test (`fitil-indicators` 30, `fitil-chart` 29, `fitil-market` 25, `fitil-app` 23, `fitil-core` 15, `fitil-store` 14, `fitil-exchange` 9, `fitil-binance` 9) ve 3 canlı test. fmt ve clippy temiz. Test penceresi `target/verify` içindeki programı kilitlediği için son çalıştırmada `--lib --bins` kullanıldı.

### Çizim araçları TradingView düzeninde: açılır menü ve 10 yeni araç (kodlandı, proje sahibinin denemesi bekleniyor)

**Nasıl başladı:** Proje sahibi TradingView'ın çizgi menüsünün ekran görüntüsünü gönderdi: "Çizim araçlarını TradingView'daki gibi yapalım. Soldaki elementlere tıklayınca sağda bir menü açılıyor, fotoğraftaki gibi, oradan seçim yapılıyor. Önce fotoğraftaki elementlerden başlayalım, sonra diğerlerine geçeriz." (karar 19)

**Araştırma:** Yeni araçların nasıl çizildiği TradingView'ın yardım sayfalarından okundu (Auto Pitchfork, Inside pitchfork, Disjoint channel, Flat top/bottom, Info line, Trend angle, Regression trend, Crossline). Sayfaların söylemediği yerlerde seçilenler:
- İç dirgenin dişleri: 1. ve 2. noktadan geçen paraleller.
- Ayrık kanalın ikinci çizgisi: 3. noktadan başlıyor, ilkiyle aynı eğimde ve uzunlukta.
- Düz tepe/dipte düz çizgi eğik çizgiyle aynı zaman aralığında.
- Regresyonda bantlar ±2 standart sapma, kaynak kapanış.

**Yapılanlar:**
1. **`docs/TASARIM.md` §4.4:** yeni araçlar, tanımları, menü düzeni, kısayollar.
2. **`fitil-core`:** `DrawingKind` 21 türe çıktı: bilgi çizgisi, trend açısı, kesişim çizgisi, regresyon trendi, düz tepe/dip, ayrık kanal ve dört dirgen.
3. **`fitil-chart`:**
   - `shapes.rs`'te yeni şekiller.
   - Bilgi çizgisinin kutusu: fiyat farkı, yüzde, mum, süre, açı.
   - Trend açısı: yay ve derece.
   - Regresyon: `regression` fonksiyonu, Pearson R etiketi.
   - Dirgenler: `pitchfork_frame`.
   - Dikey çizgi ve kesişim çizgisinin tarihi zaman ekseninde (`line_times`, `time_box`).
4. **`fitil-app`:**
   - `flyout.rs`: düğmenin yanında açılan menü. iced'in overlay yapısıyla yapıldı (`advanced` özelliği açıldı; yeni indirme yok).
   - `tools.rs`: gruplar, bölümler (Çizgiler, Kanallar, Dirgenler), Alt kısayolları.
   - `icons.rs`: çizgi araçlarının TradingView benzeri simgeleri canvas'la çiziliyor. Kullanılmayan Lucide sabitleri silindi.
   - `view.rs`: sol çubukta imleç, çizgi grubu düğmesi, dikdörtgen, Fibonacci, not, ölçüm, sonra mıknatıs, kilit, gizle, sil. Menü dışarı tıklayınca, Esc'ye basınca ya da düğmeye yeniden tıklayınca kapanıyor. Yazmaya başlamak (sembol arama) da kapatıyor.
   - Alt+T trend çizgisi, Alt+H yatay çizgi, Alt+J yatay ışın, Alt+V dikey çizgi, Alt+C kesişim çizgisi.

**Ekran denetiminde bulunanlar ve düzeltilenler:**
- **Kesikli çizgilerde fazladan çizgi:** iced bir yolun ayrı parçalarını kesikli çizerken aralarını birleştiriyordu. Kesikli ya da noktalı seçilen Fibonacci seviyeleri ve kanallarda çapraz çizgiler çıkabiliyordu (Adım 5'ten kalma). Artık uç uca eklenen çizgiler tek yol, ayrı çizgiler ayrı yol olarak çiziliyor.
- **Trend açısının yayı görünmüyordu:** Yay kılavuz (kesikli) çizgi olarak çiziliyordu, kısa parçaları kesik boyuna yetmiyordu. Artık çizimin kendi çizgisiyle çiziliyor.

**Ekranda doğrulananlar** (yeni bir test penceresi, önbellek kopyası `cache-copy2`, geçici `TEMP-VERIFY` satırlarıyla beş örnek takım):
- açık menü, bölüm başlıkları, simgeler, kısayollar ve grup düğmesinin köşe işareti;
- bilgi çizgisi kutusu ("-617.01 (-0.76%)", "25 mum, 1g 1s", "-13.30°");
- trend açısı ("-41.40°"), regresyon (Pearson R "-0.65"), kesişim çizgisinin fiyatı ve tarihi;
- düz tepe/dip (3. tutamaç düz çizginin ucunda), ayrık kanal, dikey çizginin tarihi;
- dört dirgen;
- kesikli Fibonacci, regresyon ve trend açısı.

Geçici satırlar silindi. Önceki test penceresine (pid 12000) dokunulmadı.

**Testler:** 166 test (`fitil-chart` 34, `fitil-indicators` 30, `fitil-app` 29, `fitil-market` 25, `fitil-core` 16, `fitil-store` 14, `fitil-exchange` 9, `fitil-binance` 9) ve 3 canlı test. fmt ve clippy temiz.

## 2 Kasım 2026

### Kaydırma akıcılığı: ölçüm ve mumları çizme yönteminin değişmesi (tamamlandı)

**Nasıl başladı:** Proje sahibi yeni sürümü açtıktan sonra sordu: "x ve y ekseninde grafiği çektiğim zaman TradingView kadar akıcı çalışmıyor sanki, neden olabilir?"

**Ölçüm:** Tıklanamadığı için grafik geçici kodla 8 ms'de bir yarım mum kaydırıldı ve her karenin çizim süresi ölçüldü (yöntem `04-teknik-notlar.md`, "Akıcılığı ölçmek"). Pencere 60 Hz bir ekranda açılıyor, yani her kare 16 ms'den kısa sürmeli. Eski yöntemle:

| Ekrandaki mum | Debug | Release |
|---|---|---|
| ~140 (açılıştaki görünüm) | 3,8 ms, saniyede 60 kare | 0,4 ms |
| ~560 (uzaklaştırınca) | ortalama 27 ms, en kötü 53 ms; saniyede 28–50 kare | 2,3 ms, saniyede 60 kare |

**Bulunan nedenler:**
1. Proje sahibine debug sürüm açılıyordu. Çizimde yaklaşık 10 kat yavaş. Takılmanın ana nedeni bu: zaman eksenini sürükleyince mumlar sıkışıyor, ekrandaki mum sayısı artıyor ve debug sürüm saniyede 30 kare civarına düşüyor.
2. Mumlar ve hacim çubukları tek büyük yol olarak dolduruluyordu. lyon'un genel doldurma algoritması parça sayısı arttıkça orantısız yavaşlıyor.

**Yapılanlar:**
- `fitil-chart/src/draw.rs`: mumların gövdesi ve fitili, hacim çubukları, OHLC çubukları ve histogram sütunları tek tek `fill_rectangle` ile çiziliyor. Parabolic SAR gibi noktalar tek tek daire. Görünüş aynı: fitil yine tam piksele oturan 1 piksellik sütun, OHLC'nin çizgileri eski 1 piksellik çizgilerle aynı yeri kaplıyor. Mumun rengi `candle_color` yardımcısında toplandı.
- Proje sahibine artık release sürüm açılıyor (`target\release\fitil.exe`).

**Yeni yöntemle ölçüm** (hepsi saniyede 60 kare):

| Ekrandaki mum | Debug | Release |
|---|---|---|
| ~140 | 1,1 ms | 0,2 ms |
| ~560 | 2 ms | 0,4 ms |
| ~1.130 (en uzak görünüm) | 3,7 ms | 0,8 ms |

**Ekranda doğrulananlar** (test penceresi, önbellek kopyası `perf-data`, geçici `TEMP-VERIFY` satırlarıyla MACD, Hacim ve Parabolic SAR eklendi): mumlar, OHLC çubukları, hacim çubukları, MACD histogramı, Hacim paneli ve SAR noktaları. Büyütülmüş görüntüde fitiller gövdelerin ortasında, OHLC'de açılış solda, kapanış sağda. Geçici satırlar silindi.

**Ölçülemeyen:** Gerçek fare sürüklemesi. Her fare hareketinde pencerenin tamamı yeniden kuruluyor, debug'da bu da yavaş. Proje sahibi release sürümde dener.

**Dikey sürükleme:** Grafiği dikey çekerken ilk 12 piksel bilerek yok sayılıyor (`VERTICAL_DRAG_THRESHOLD`, yatay kaydırırken grafik yanlışlıkla oynamasın diye). Proje sahibine, TradingView'da böyle bir bekleme yoksa kaldırılabileceği söylendi.

**Testler:** 166 test ve 3 canlı test. fmt ve clippy temiz. Commit `ec71f3b`, GitHub'da derleme üç sistemde geçti.

**Proje sahibinin denemesi:** Release sürümde grafiği kaydırdı ve "Evet, akıcı" dedi.
