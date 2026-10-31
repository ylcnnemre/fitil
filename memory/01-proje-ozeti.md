# Proje Özeti ve Kararlar

## Fitil nedir?

Fitil, kripto para piyasaları için açık kaynak ve ücretsiz bir masaüstü grafik uygulamasıdır. TradingView'a alternatif olarak doğdu. Proje sahibi TradingView'ın ücretli kısıtlarından (indikatör sınırı, tek grafik düzeni, özel zaman dilimleri vb.) bıkmıştı.

- Herkes GitHub'dan indirip kendi bilgisayarında çalıştırır.
- Uygulama borsanın herkese açık API'sine doğrudan bağlanır. Arada sunucu yoktur.
- API anahtarı, hesap ve üyelik gerekmez.

İsim, mum grafiğindeki iğneye verilen "fitil" adından geliyor.

## Proje sahibi ve çalışma şekli

- **İletişim:** Proje sahibi Türkçe konuşur. Teknik konular sade bir dille, gereksiz terim kullanmadan anlatılır.
- **Roller:** Proje sahibi **Rust bilmez**. Kodun tamamını Claude yazar. Proje sahibi ne yapılacağına karar verir, her adımda çalışan uygulamayı dener ve geri bildirim verir.
- **Doğrulama:** Proje sahibi kodu okumadan ilerlemeyi görebilmelidir. Her adımın sonunda çalıştırılabilir bir uygulama ve GitHub'da otomatik derleme olur.
- **Onay gereken işler:** İndirme, kurulum, herkese açık yayın (repo açma, sürüm çıkarma) ve kapsam değişikliği gibi işlerden önce proje sahibine sorulur.
- **Kodlamadan önce konuşma:** Proje sahibi kodlamadan önce her şeyin ayrıntılı konuşulmasını istedi. Bu yüzden önce `docs/TASARIM.md` yazıldı ve onaylandı.
- **Hafıza:** Proje sahibi, yeni oturumlarda hiçbir şeyin unutulmaması için bu `memory/` klasörünün tutulmasını istedi.

## Kararlar

Tüm kararlar 30 Ekim 2026'da, ilk oturumda verildi.

| # | Karar | Neden / not |
|---|---|---|
| 1 | Masaüstü uygulama, sunucusuz. Her kullanıcı borsaya doğrudan bağlanır. | Sunucu masrafı ve bakımı yok. İstek limitleri her kullanıcının kendi IP'sine işler. Veriyi biz dağıtmadığımız için veri lisansı sorunu yok. |
| 2 | **Tam native Rust** | Proje sahibi hafiflik ve hız için Rust istedi. Claude ilk olarak Electron + TypeScript önermişti. Tauri de değerlendirildi ama orada grafik yine JavaScript ile çizileceği için elendi. Tam native Rust: küçük tek dosya, düşük RAM, grafikler doğrudan GPU ile çiziliyor. |
| 3 | Arayüz kütüphanesi: **iced** (0.14) | Çoklu grafik düzeni için bölmeli yerleşim (pane_grid) hazır geliyor, GPU ile çiziyor. Aynı alanda yazılmış açık kaynak Flowsurface uygulaması ve System76'nın COSMIC masaüstü iced kullanıyor. Alternatif olarak egui düşünüldü. |
| 4 | **Al-sat yok**, emir gönderme yok, API anahtarı yok | Proje sahibinin kararı. Fitil sadece analiz aracıdır. Bu tür özellikler önerilmemeli. |
| 5 | Kapsam: grafik, çizim araçları, sınırsız indikatör. Backtest daha sonra. | Proje sahibinin kararı. |
| 6 | Açık kaynak, **GPL-3.0** lisansı | Kimse projeyi alıp kapalı, ücretli bir ürüne çeviremesin diye. MIT/Apache seçeneği de anlatıldı, proje sahibi GPL-3.0'ı seçti. |
| 7 | Arayüz dili Türkçe ve İngilizce, yeni dil eklenebilir | Çeviri altyapısı Fluent (.ftl dosyaları). Yeni dil eklemek için Rust bilmek gerekmez. |
| 8 | Platformlar: Windows, macOS, Linux | Derlemeyi GitHub Actions yapar. macOS bu bilgisayarda denenemez, gerçek bir Mac'te deneyecek gönüllü gerekecek. |
| 9 | İsim: **Fitil** | Claude önerdi, proje sahibi kabul etti. Klasörün adı `tradingclone` ama ürün adı Fitil. |
| 10 | İlk borsa: **Binance** (Spot ve USDT-M perpetual) | Diğer borsalar (Bybit, OKX, BtcTurk) Faz 2'de. |
| 11 | Kod ve kod içi açıklamalar İngilizce. README Türkçe ve İngilizce. Belgeler ve bu klasör Türkçe. | Uluslararası katkı için kod İngilizce. Proje sahibi belgeleri okuyabilsin diye belgeler Türkçe. |
| 12 | Telemetri yok | Fitil kullanıcı hakkında veri göndermez. Sadece borsaya ve sürüm kontrolü için GitHub'a bağlanır. |
| 13 | GitHub deposu `ylcnnemre/fitil`, herkese açık | Proje sahibinin GitHub hesabı `ylcnnemre`. Bu bilgisayarda `gh` ile oturum açık. |
| 14 | Dışarıdan gelen katkıları Claude inceleyip Türkçe özetler, birleştirme kararı proje sahibinde | Proje sahibi Rust bilmediği için. Kripto projelerinde kötü niyetli katkı riski var. |
| 15 | Mimari: "port ve adaptör" (hexagonal), çok borsalı ve modüler (31 Ekim 2026) | Proje sahibi sordu: "Mimariyi pek konuşmadık, temiz bir mimari daha iyi olur. Hyperliquid, OKX, Huobi gibi borsalar ve farklı modüller gelebilir, hepsini düşünerek hareket edelim." Kurallar `docs/TASARIM.md` §6'da. Her borsa ayrı bir adaptör crate'idir ve ortak `Exchange` arayüzünü uygular. İş mantığı `fitil-market`'te, arayüzsüz test edilir. |
| 16 | Eklenecek borsalar: OKX, Hyperliquid, HTX (Huobi), Bybit, BtcTurk | Faz 2. Her biri §6.3'teki adımlarla eklenir. |
| 17 | Arayüz TradingView düzeninde: solda sembol listesi yok, ekran grafiğe ayrılır, sembol bir arama penceresinden seçilir (31 Ekim 2026) | Proje sahibi TradingView'ın ekran görüntüsünü gönderip şunu dedi: "Arayüz TradingView gibi olmalı, solda coinlerin listelenmesine gerek yok, coin aratınca modal açılsın." İzleme listesi (Adım 6) sağda açılıp kapanan bir panel olacak. `docs/TASARIM.md` §4.5. |

## Fazlar (özet)

Ayrıntılar `docs/TASARIM.md` dosyasında.

- **Faz 1 (v0.1):**
  - Binance verisi, canlı mumlar, özel zaman dilimleri, sınırsız geçmiş.
  - Yaklaşık 20 indikatör ve indikatör şablonları.
  - Çizim araçları.
  - Çoklu grafik, izleme listesi.
  - Grafik görüntüsü alma, yedekleme.
  - Türkçe ve İngilizce arayüz, koyu ve açık tema.
- **Faz 2:**
  - Alarmlar, bar replay.
  - Lua ile kendi indikatörünü yazma.
  - Çoklu pencere.
  - Diğer borsalar, vadeli işlem verileri.
- **Faz 3:**
  - Backtest.
  - Volume profile, footprint, emir defteri ısı haritası.
  - Pine'a benzeyen kendi script dilimiz.

## Faz 1 yol haritası (7 adım)

| # | Adım |
|---|---|
| 1 | Geliştirme ortamı, proje iskeleti, GitHub deposu, otomatik derleme ve lisans kontrolü, topluluk dosyaları, ikon taslağı |
| 2 | Binance bağlantısı ve yerel depo (SQLite) |
| 3 | Grafik motoru (tek grafik, canlı mumlar, yakınlaştırma, kaydırma) |
| 4 | İndikatörler ve paneller |
| 5 | Çizim araçları |
| 6 | Çalışma alanı (çoklu grafik, izleme listesi, sembol arama, düzen kaydetme) |
| 7 | Dil, tema, ayarlar, yedekleme, paketleme → v0.1 yayını |
