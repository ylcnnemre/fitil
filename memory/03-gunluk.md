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
  - Önbellek konumu: Windows'ta `%LOCALAPPDATA%\Fitil\cache.sqlite`. `docs/TASARIM.md` §6.3 buna göre güncellendi.
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
