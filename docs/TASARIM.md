# Fitil: Tasarım Belgesi

> **Durum:** Taslak, onay bekliyor
> **Tarih:** 30 Ekim 2026
> **Lisans:** GPL-3.0

Bu belge, kodlamaya başlamadan önce üzerinde anlaştığımız her şeyi tek yerde topluyor. Bir şey değişirse önce burası güncellenir, kod sonra yazılır.

---

## 1. Amaç

Fitil, kripto para piyasaları için açık kaynak ve ücretsiz bir masaüstü grafik uygulamasıdır. TradingView'daki ücretli kısıtlar (indikatör sınırı, tek grafik düzeni, özel zaman dilimleri vb.) Fitil'de yoktur. Herkes GitHub'dan indirip kendi bilgisayarında çalıştırır.

## 2. Temel ilkeler

1. **Sunucu yok.** Uygulama borsaya doğrudan bağlanır. Arada bizim sunucumuz olmadığı için masraf da bakım da yoktur.
2. **Hesap ve API anahtarı yok.** Uygulama sadece borsanın herkese açık piyasa verisini okur. Üyelik veya giriş gerekmez.
3. **Al-sat yok.** Fitil bir analiz aracıdır. Emir gönderme özelliği hiçbir fazda yoktur.
4. **Sınır yok.** İndikatör, grafik, çizim ve izleme listesi sayısında yapay bir sınır konmaz.
5. **Hafif ve hızlı.** Uygulama Rust ile native olarak yazılır, grafikler ekran kartıyla (GPU) çizilir.
6. **Herkes için.** Açık kaynak (GPL-3.0) olur, Windows, macOS ve Linux'ta çalışır, Türkçe ve İngilizce arayüzle gelir, yeni dil eklenebilir.
7. **Veri kullanıcıda kalır.** Çizimler, düzenler ve ayarlar sadece kullanıcının bilgisayarında saklanır.
8. **Telemetri yok.** Fitil kullanıcı hakkında hiçbir veri toplamaz ve göndermez. Yalnızca borsaya ve yeni sürüm kontrolü için GitHub'a bağlanır.

## 3. Bilinçli olarak kapsam dışı

| Konu | Neden |
|---|---|
| Emir gönderme, API anahtarı | Proje kararı: Fitil sadece analiz aracıdır. |
| Hesap, bulut senkronizasyonu | Sunucu gerektirir. İlke 1'e aykırı. |
| Sosyal özellikler (fikir paylaşımı, yorumlar) | Sunucu gerektirir, kapsam dışı. |
| Pine Script uyumluluğu | Pine, TradingView'a ait tescilli bir dil. Fitil'in kendi script sistemi olacak (Faz 2). |
| Kripto dışı piyasalar (BIST, hisse, forex) | Borsa API'leri yalnızca kripto verir. Diğer piyasalar ücretli veri kaynakları gerektirir. Şimdilik yok. |

---

## 4. Faz 1: İlk sürüm (v0.1) kapsamı

### 4.1 Veri kaynağı: Binance

- **Piyasalar:** Binance Spot ve Binance USDT-M vadeli (perpetual). COIN-M vadeli sonraki fazlara kalır.
- **Sembol listesi:** Tüm işlem gören çiftler borsadan otomatik çekilir. Fiyat hassasiyeti (kaç basamak) her sembolün kendi bilgisinden alınır.
- **Borsanın kendi zaman dilimleri:** 1s (yalnız spot), 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M.
- **Özel zaman dilimleri:** Kullanıcı istediği periyodu yazar (2m, 7m, 45m, 3h, 2d…). Fitil bunu, ona tam bölünen en büyük hazır zaman diliminden üretir. Örneğin 45m, 15m mumlardan; 3h, 1h mumlardan üretilir. Böylece gereksiz yere milyonlarca 1 dakikalık mum indirilmez.
- **Canlı veri:** Açık mum ve izleme listesindeki fiyatlar WebSocket ile anlık güncellenir.
- **Geçmiş veri:** Grafik sola kaydırıldıkça eski mumlar parça parça indirilir (Binance tek istekte en fazla 1000 mum veriyor). İnen her mum yerel veritabanına yazılır, aynı veri bir daha indirilmez.
- **Kopmalara dayanıklılık:**
  - Binance bağlantıyı 24 saatte bir kapatır, internet de kesilebilir. Fitil otomatik olarak yeniden bağlanır.
  - Aradaki eksik mumları REST ile tamamlar. Grafikte boşluk ("delik") oluşmaz.
- **İstek limiti:**
  - Binance her yanıtta o dakika ne kadar istek "ağırlığı" kullanıldığını bildirir.
  - Fitil bunu takip eder ve limite yaklaşınca kendiliğinden yavaşlar.
- **Erişim engeli:** Borsa bazı ülkelerden erişimi engelliyorsa kullanıcıya anlaşılır bir hata mesajı gösterilir.
- **Proxy:** Şirket ağları ve borsaya erişimin engelli olduğu yerler için HTTP ve SOCKS5 proxy ayarı vardır.
- **Saat düzeltmesi:** Bilgisayarın saati birkaç saniye kaymış olabilir. Mum kapanış geri sayımı yine de doğru olsun diye zaman Binance'in sunucu saatine göre düzeltilir.
- **Günlük mumun saati:** Binance'te günlük mum UTC 00:00'da, yani Türkiye saatiyle 03:00'te açılıp kapanır. Fitil bunu değiştirmez, böylece mumlar herkesin baktığı mumlarla aynı olur.

### 4.2 Grafik

- **Grafik tipleri:** Mum, OHLC çubuk, çizgi, alan, Heikin Ashi.
- **Gezinme:**
  - Fare tekerleğiyle yakınlaştırma, sürükleyerek kaydırma.
  - Fiyat ve zaman eksenlerini sürükleyerek ölçekleme.
  - Çift tıklayınca otomatik ölçeğe dönüş.
- **Fiyat ölçeği:** Otomatik, logaritmik ve yüzde seçenekleri.
- **Artı imleci:** Üzerine gelinen mumun açılış, en yüksek, en düşük, kapanış ve hacim bilgisini gösterir.
- **Son fiyat:** Son fiyat çizgisi ve etiketi, yanında mumun kapanmasına kalan süre.
- **Hacim:** Ana grafiğin altında bindirme veya ayrı panel olarak gösterilebilir.
- **Paneller:** İndikatörler için alt paneller açılabilir. Panellerin yüksekliği sürüklenerek ayarlanır.
- **Saat dilimi:** Varsayılan olarak bilgisayarın saat dilimi kullanılır. Ayarlardan değiştirilebilir.

### 4.3 İndikatörler

Sayı sınırı yok. Her indikatörün periyodu, kaynağı (kapanış, hl2 vb.), rengi, kalınlığı ve görünürlüğü ayarlanabilir.

| Grup | İndikatörler |
|---|---|
| Hareketli ortalamalar | SMA, EMA, WMA, VWMA |
| Bantlar ve kanallar | Bollinger Bantları, Keltner Kanalı, Donchian Kanalı |
| Trend | SuperTrend, Parabolic SAR, ADX / DMI, Ichimoku |
| Momentum | RSI, MACD, Stokastik, Stokastik RSI, CCI |
| Volatilite | ATR |
| Hacim | Hacim, Alış/Satış Hacmi, CVD (kümülatif hacim deltası), OBV, VWAP (oturum bazlı) |

Yeni mum geldiğinde indikatörün tamamı yeniden hesaplanmaz, sadece son kısım güncellenir. Böylece çok sayıda indikatör açıkken bile uygulama hızlı kalır.

Alış/Satış Hacmi ve CVD için ek veri indirmek gerekmez. Binance her mumla birlikte "piyasa alıcılarının hacmi" bilgisini zaten gönderiyor, bu ikisi oradan hesaplanır.

**İndikatör şablonları:** Kullanıcı bir indikatör setini ayarlarıyla birlikte bir isimle kaydeder (örneğin "EMA 20/50/200 + RSI + Hacim"). Kaydettiği seti tek tıkla başka bir grafiğe uygular.

### 4.4 Çizim araçları

- **Çizgiler:** Trend çizgisi, ışın, uzatılmış çizgi, yatay çizgi, yatay ışın, dikey çizgi.
- **Şekiller:** Dikdörtgen, paralel kanal.
- **Fibonacci:** Fibonacci düzeltme. Seviyeleri düzenlenebilir.
- **Diğer:** Metin notu, ölçüm aracı (fiyat farkı, yüzde değişim, mum sayısı ve süre).
- **Mıknatıs modu:** Çizimin uçları mumun açılış, en yüksek, en düşük veya kapanış noktasına yapışır.
- **Düzenleme:**
  - Seçme, taşıma, uç noktalarını sürükleme, kopyalama ve silme.
  - Kilitleme ve gizleme.
  - Geri al / yinele (Ctrl+Z / Ctrl+Y).
- **Stil:** Renk, kalınlık, çizgi tipi (düz, kesikli, noktalı).
- **Kayıt:**
  - Çizimler sembole bağlıdır ve tüm zaman dilimlerinde görünür.
  - Otomatik kaydedilir, uygulama kapansa da kaybolmaz.

### 4.5 Çalışma alanı

- **Çoklu grafik düzeni:** Tek grafik, 2'li (yan yana veya üst üste), 3'lü, 2×2, 6'lı ve 8'li düzen. Bölmeler sürüklenerek boyutlandırılır.
- **Senkronizasyon:** Grafikler arasında sembol, zaman dilimi, artı imleci ve zaman ekseni isteğe bağlı olarak eşlenebilir.
- **İzleme listesi:**
  - Canlı fiyat, 24 saatlik değişim yüzdesi ve hacim.
  - Birden fazla liste oluşturulabilir, sütuna göre sıralanabilir.
  - Bir sembole tıklanınca seçili grafikte açılır.
- **Sembol arama:** Grafik üzerinde yazmaya başlayınca arama penceresi açılır.
- **Kaldığın yerden devam:** Uygulama kapanıp açıldığında düzen, grafikler, indikatörler ve çizimler aynen geri gelir. Düzenler isim verilerek kaydedilebilir.
- **Grafik görüntüsü:** Tek tuşla grafiğin resmi panoya kopyalanır ya da PNG olarak kaydedilir.
- **Yedekle / geri yükle:** Çizimler, düzenler, şablonlar ve ayarlar tek bir dosyaya aktarılıp başka bir bilgisayara taşınabilir.
- **Görünüm:**
  - Koyu ve açık tema. Mum renkleri özelleştirilebilir.
  - Renk körleri için hazır mavi/turuncu mum renkleri.
  - 4K ekranlarda ve Windows'un %125–150 ölçeklemesinde net görünüm. Arayüz ve yazı boyutu ayrıca büyütülebilir.
- **Dil:**
  - Türkçe ve İngilizce, uygulama içinden anında değiştirilebilir. İlk açılışta bilgisayarın dili seçilir.
  - Sayı biçimi (1.234,56 ya da 1,234.56) ayrı bir ayardır.
- **Kısayollar:** Sık kullanılan işlemler için klavye kısayolları.

### 4.6 Güvenilirlik ve kaynak kullanımı

- **Güncellemede veri kaybı olmaz:** Kaydedilen her çizim, düzen ve ayar, hangi biçimde kaydedildiğini gösteren bir sürüm numarası taşır. Yeni bir Fitil sürümü eski kayıtları otomatik olarak yeni biçime dönüştürür.
- **Önbellek yönetimi:**
  - Mum önbelleğinin ayarlanabilir bir boyut sınırı vardır. Sınır aşılınca en uzun süredir bakılmayan veriler silinir.
  - Ayarlarda "Önbelleği temizle" düğmesi bulunur.
  - Çizimler, düzenler ve ayarlar önbellek temizliğinden etkilenmez.
- **Arka planda az kaynak:** Pencere küçültüldüğünde ya da görünmezken grafik çizimi durur, sadece veri akışı devam eder. Bu, dizüstü bilgisayarlarda pil ömrü için önemlidir.
- **Hata kaydı ve bildirimi:**
  - Uygulama hataları bilgisayardaki bir kayıt (log) dosyasına yazar.
  - "Hata bildir" düğmesi GitHub'da bir hata sayfası açar. Sayfada sürüm ve işletim sistemi bilgileri önceden doldurulmuş olur.
  - Kullanıcı ne gönderdiğini görür ve gönderme kararını kendisi verir.

### 4.7 Faz 1 başarı ölçütleri

v0.1 yayınlanmadan önce şunların sağlanması hedefleniyor:

- [ ] Uygulama 1 saniyenin altında açılıyor.
- [ ] 4 grafik açık, her birinde 5 indikatör varken ve canlı veri akarken kaydırma ve yakınlaştırma akıcı.
- [ ] 4 grafik açıkken bellek kullanımı 150 MB'ın altında.
- [ ] 24 saat açık kalan bir grafikte eksik mum yok.
- [ ] İnternet kesilip geri geldiğinde uygulama kendi kendine toparlanıyor.
- [ ] Windows, macOS ve Linux sürümleri GitHub'da otomatik üretiliyor.

---

## 5. Sonraki fazlar

**Faz 2: TradingView'da ücretli olan özellikler**
- **Alarmlar:** Fiyat, çizgi kesişimi veya indikatör koşuluyla kurulur. Masaüstü bildirimi ve ses ile uyarır, Telegram bildirimi isteğe bağlıdır. Uygulama sistem tepsisinde arka planda çalışabilir.
- **Bar replay:** Geçmişi mum mum yeniden oynatma.
- **Kendi indikatörünü yazma:** Uygulamanın içinde Lua tabanlı script editörü.
- **İndikatör üstüne indikatör:** Örneğin RSI'ın hareketli ortalaması.
- **Çoklu pencere:** Birden fazla pencere ve ikinci monitör desteği.
- **Diğer borsalar:** Bybit, OKX, BtcTurk.
- **Karşılaştırma:** Sembol karşılaştırma, BTC/ETH gibi oran grafikleri.
- **Vadeli işlem verileri:** Funding oranı, açık pozisyon (OI), likidasyonlar.
- **Genişleme:** Daha fazla çizim aracı ve indikatör, yeni arayüz dilleri.

**Faz 3: İleri seviye**
- Strateji backtest.
- Volume profile, footprint grafiği, emir defteri ısı haritası.
- Pine'a benzeyen, trader'ların kolay öğreneceği kendi script dilimiz.

**Zamanı belli olmayanlar**
- Kullanıcı kılavuzu ve basit bir tanıtım sitesi (GitHub Pages ile ücretsiz).
- Klavye kısayollarını özelleştirme.
- Kurulumsuz (portable) sürümde verilerin programın yanındaki klasörde tutulması.
- Sağdan sola yazılan diller (Arapça, Farsça vb.). iced'in bu konudaki desteği sınırlı, eklenirse ek iş çıkar.

---

## 6. Teknik mimari

```
┌────────────────────────── Fitil ───────────────────────────┐
│                                                            │
│  Arayüz (iced)                                             │
│  pencere · grafik bölmeleri · izleme listesi · ayarlar     │
│        │                     ▲                             │
│        ▼                     │                             │
│  Grafik motoru (GPU)    İndikatör motoru                   │
│  mumlar · eksenler      artımlı hesaplama                  │
│  çizim araçları                                            │
│        ▲                     ▲                             │
│        └──────────┬──────────┘                             │
│              Veri katmanı                                  │
│   borsa adaptörleri · WebSocket yöneticisi · istek limiti  │
│                   │                                        │
│              Yerel depo (SQLite)                           │
│   mum önbelleği · çizimler · düzenler · ayarlar            │
└───────────────────┬────────────────────────────────────────┘
                    │ doğrudan (REST + WebSocket)
                 Binance
```

### 6.1 Kod yapısı

Proje, birbirinden bağımsız test edilebilen parçalara bölünür (Cargo workspace):

```
fitil/
├─ crates/
│  ├─ fitil-core/        ortak veri tipleri: mum, sembol, zaman dilimi
│  ├─ fitil-exchange/    borsa adaptörleri (önce Binance)
│  ├─ fitil-store/       SQLite önbellek ve kayıtlar
│  ├─ fitil-indicators/  indikatör hesaplamaları
│  ├─ fitil-chart/       grafik motoru ve çizim araçları
│  └─ fitil-app/         uygulamanın kendisi, arayüz
├─ locales/
│  ├─ tr/                Türkçe metinler
│  └─ en/                İngilizce metinler
├─ docs/                 belgeler (bu dosya dahil)
└─ .github/workflows/    otomatik derleme ve test
```

Her borsa aynı ortak arayüzü uygular. Yeni bir borsa eklemek, uygulamanın geri kalanına dokunmadan yeni bir adaptör yazmak demektir.

### 6.2 Teknoloji seçimleri

| Konu | Seçim | Not |
|---|---|---|
| Dil | Rust | |
| Arayüz | iced | Bölmeli yerleşim (pane_grid) hazır geliyor. Grafikler iced'in GPU çizim katmanıyla çizilir. |
| Ağ | tokio, tokio-tungstenite, reqwest | Çok sayıda eşzamanlı bağlantıyı az kaynakla yönetir. |
| Veritabanı | SQLite (rusqlite) | Tek dosya, kurulum gerektirmez. |
| Çeviri | Fluent (.ftl dosyaları) | Yeni dil eklemek için Rust bilmek gerekmez, bir metin dosyasını çevirmek yeterli. |
| Yazı tipi | Inter (SIL Open Font License) | Türkçe karakterleri tam destekler. Uygulamayla birlikte gelir. |
| Zaman | İçeride her şey UTC | Ekranda kullanıcının saat diliminde gösterilir. |
| Fiyatlar | 64 bit ondalık sayı | Ekranda sembolün kendi hassasiyetiyle gösterilir. |
| Kullanıcı scriptleri (Faz 2) | Lua | Hızlı, küçük ve öğrenmesi kolay. |

### 6.3 Verilerin saklandığı yer

Her işletim sisteminin standart uygulama klasörü kullanılır:
- **Windows:** `%APPDATA%\Fitil`
- **macOS:** `~/Library/Application Support/Fitil`
- **Linux:** `~/.local/share/fitil`

---

## 7. Kalite ve test

- **İndikatör testleri:** İndikatörler bilinen referans değerlerle otomatik test edilir.
- **Borsa adaptörü testleri:** Kaydedilmiş gerçek Binance yanıtlarıyla test edilir, testler internete bağlı değildir.
- **Otomatik kontroller:** Her değişiklikte GitHub Actions şunları çalıştırır:
  - Windows, macOS ve Linux'ta derleme,
  - testler,
  - kod biçimi kontrolü (`cargo fmt`),
  - hata yakalayıcı (`cargo clippy`),
  - lisans kontrolü (`cargo deny`): kullanılan her kütüphanenin GPL-3.0 ile uyumlu olduğu doğrulanır.
- **Kod dili:** Kod ve kod içi açıklamalar İngilizcedir (uluslararası katkı için). README Türkçe ve İngilizcedir.

## 8. Dağıtım

- **GitHub Releases:** Her sürümde otomatik olarak şunlar yüklenir:
  - **Windows:** Kurulum dosyası ve kurulumsuz (portable) .zip.
  - **macOS:** .dmg (Apple Silicon ve Intel).
  - **Linux:** AppImage ve .deb.
- **Doğrulama:** Her dosyanın yanında bir SHA-256 özeti yayınlanır. Bu özet, indirilen dosyanın değiştirilmediğini doğrulamaya yarayan bir parmak izidir.
- **Güncelleme:** Uygulama açılışta GitHub'da yeni sürüm olup olmadığına bakar ve kullanıcıya bildirir. Otomatik kurulum sonraki sürümlere kalır.
- **İmzalama:**
  - **Windows:** Açık kaynak projelere ücretsiz sertifika veren SignPath Foundation'a başvurulacak. O zamana kadar ilk açılışta bir uyarı çıkar.
  - **macOS:** Apple geliştirici hesabı (yıllık 99 $) alınana kadar kullanıcının ilk açılışta bir kez izin vermesi gerekir. Nasıl yapılacağı README'de anlatılır.

## 9. Yasal uyarı ve marka

- README'de ve uygulamanın "Hakkında" ekranında şunlar yazar:
  - Fitil yatırım tavsiyesi vermez.
  - Verilerin doğruluğu ve kesintisizliği garanti edilmez.
  - Fitil'in Binance veya başka bir borsayla resmi bir bağı yoktur.
- Borsa logoları kullanılmaz, borsaların sadece adı yazılır.
- Binance API kullanım koşullarına uyulur.
- "TradingView" adı, logosu ya da arayüzünün birebir kopyası kullanılmaz.

## 10. Açık kaynak ve topluluk

- **Sahte kopyalara karşı önlem:**
  - Tek resmi indirme yeri GitHub Releases'tir. README bunu açıkça yazar ve başka yerlerden indirilen kopyalara karşı uyarır.
  - Her dosyanın SHA-256 özeti yayınlanır ve dosyalar imzalanır (bkz. Bölüm 8).
- **Dışarıdan gelen katkılar:**
  - Topluluktan gelen her kod değişikliğini (pull request) Claude inceler ve proje sahibine Türkçe özetler. Riskli bir şey varsa açıkça belirtir.
  - Birleştirme kararı proje sahibindedir.
  - Hiçbir katkı incelenmeden birleştirilmez.
- **Lisans uyumluluğu:** Kullanılan her kütüphane ve yazı tipi GPL-3.0 ile uyumlu olmalıdır. Bu, her derlemede otomatik kontrol edilir (bkz. Bölüm 7).
- **Logo ve ikon:** İlk ikon taslağını Claude hazırlar. Daha sonra bir tasarımcı ya da topluluk geliştirebilir.
- **Topluluk dosyaları:**
  - `CONTRIBUTING.md`: katkı rehberi.
  - Çeviri rehberi: yeni bir dilin nasıl ekleneceği.
  - Hata bildirimi ve özellik isteği şablonları.
  - `CHANGELOG.md`: sürüm notları.
- **Sürüm numaraları:** 0.1.0, 0.2.0 diye ilerler. 1.0.0, uygulama kararlı kabul edildiğinde çıkar.

## 11. Çalışma şekli ve yol haritası

**Roller:** Kodun tamamını Claude yazar. Proje sahibi her aşamada çalışan uygulamayı dener, geri bildirim verir ve kararları onaylar.

Faz 1, her biri çalıştırılıp denenebilir bir sonuç veren adımlarla ilerler:

| # | Adım | Sonunda ne görülür |
|---|---|---|
| 1 | Geliştirme ortamı, proje iskeleti, GitHub deposu, otomatik derleme ve lisans kontrolü, topluluk dosyaları, ikon taslağı | Üç platformda derlenen boş bir Fitil penceresi |
| 2 | Binance bağlantısı ve yerel depo | Sembol listesi, geçmiş mumlar ve canlı fiyatlar akıyor |
| 3 | Grafik motoru | Tek grafik: canlı mumlar, yakınlaştırma, kaydırma, artı imleci |
| 4 | İndikatörler ve paneller | İndikatör ekleme ve ayarlama |
| 5 | Çizim araçları | Çizim yapma, düzenleme, kaydetme |
| 6 | Çalışma alanı | Çoklu grafik, izleme listesi, sembol arama, düzen kaydetme |
| 7 | Dil, tema, ayarlar, yedekleme, paketleme | **v0.1 yayını** |

## 12. Riskler

| Risk | Önlem |
|---|---|
| Binance API'sinin değişmesi veya bölgesel erişim engeli | Adaptör yapısı sayesinde değişiklik tek yerde düzeltilir. Faz 2'de başka borsalar eklenir. |
| macOS'un bu geliştirme bilgisayarında denenememesi | Otomatik derleme ve testler macOS'ta da çalışır. Arayüzün gerçek bir Mac'te denenmesi için gönüllü aranacak. |
| iced'in henüz 1.0 sürümüne ulaşmamış olması | Kullanılan sürüm sabitlenir. Güncellemeler bilinçli olarak, test edilerek yapılır. |
| Eski veya ekran kartı sürücüsü sorunlu bilgisayarlar | Test edilir. Gerekirse yazılımsal çizim yedeği değerlendirilir. |
| Fitil'in virüslü sahte kopyalarının dağıtılması | Tek resmi indirme yeri, SHA-256 özetleri, imzalama ve README'de uyarı (bkz. Bölüm 10). |
| Kötü niyetli kod katkısı | Her katkı incelenip özetlenir. İncelenmeden birleştirme yapılmaz. |
