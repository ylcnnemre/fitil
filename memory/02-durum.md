# Şu Anki Durum

**Son güncelleme:** 1 Kasım 2026

## Neredeyiz?

- **Tasarım:** `docs/TASARIM.md` onaylı. Mimari bölümü (§6) 31 Ekim'de yeniden yazıldı. Adım 4 ile tablo ve §6.6 (veri dosyaları), TradingView düzeniyle §4.5, Adım 5 ile §4.4 ve §6.5 (simgeler) güncellendi.
- **Adım 1 tamamlandı:** iskelet, GitHub deposu, CI.
- **Adım 2 tamamlandı:** Binance bağlantısı ve yerel önbellek.
- **Adım 3 tamamlandı:** grafik motoru.
- **Mimari düzenleme tamamlandı:** çok borsalı, port/adaptör düzeni.
- **Adım 4 tamamlandı:** 22 indikatör, alt paneller, şablonlar, ayar pencereleri.
- **TradingView düzeni tamamlandı (karar 17):** solda liste yok, sembol arama penceresi.
- **Adım 5 kodlandı, proje sahibinin denemesi bekleniyor:** çizim araçları.
  - Simgeler Lucide setinden (karar 18), `assets/fonts/lucide.ttf`.
  - `fitil-core`: çizim modeli (`Drawing`, `DrawingKind`, `Anchor`, `LineStyle`, `Rgb`).
  - `fitil-store`: `user.sqlite` göç 2, `drawings` tablosu (sembol başına).
  - `fitil-market`: `load_drawings`, `save_drawings`.
  - `fitil-chart`: çizme, seçme, taşıma, geri al/yinele, mıknatıs; çizimler her karede çizilen katmanda.
  - `fitil-app`: sol araç çubuğu, seçili çizimin satırı, geri al/yinele düğmeleri, kısayollar.
  - Ayrıntılar `03-gunluk.md` dosyasında.

Uygulama şu an şunları gösteriyor:
- **Üstte araç çubuğu:**
  - Sembol adı (büyüteç simgesiyle). Tıklayınca sembol arama penceresi açılır. Grafikte yazmaya başlamak da açar.
  - Sık kullanılan zaman dilimleri (1m 5m 15m 1h 4h 1d 1w) ve "Diğer" listesi.
  - Grafik tipi, Log, %, **İndikatörler**, **Hacim**, geri al ve yinele simgeleri.
  - En sağda dil düğmeleri.
- **Solda çizim araçları çubuğu:**
  - Seç ve taşı, trend çizgisi, ışın, uzatılmış çizgi, yatay çizgi, yatay ışın, dikey çizgi.
  - Dikdörtgen, paralel kanal, Fibonacci.
  - Metin notu, ölçüm.
  - Mıknatıs, tümünü kilitle, çizimleri gizle, tümünü sil (iki tıkla).
- **Grafik ekranın geri kalanını kaplar.** Açılışta BTCUSDT açılır.
  - Araç çubuğunun altında indikatör etiketleri.
  - Bir çizim seçiliyken onun satırı da çıkar: renk, kalınlık, çizgi tipi, metin ya da Fibonacci seviyeleri, kilitle, kopyala, sil.
  - Yatay çizgilerin fiyatı eksende görünür.
- **Sembol arama penceresi:**
  - Tümü / Spot / Vadeli sekmeleri.
  - Sonuçlar önce yakınlığa, sonra dolar hacmine göre sıralanır.
  - Her sonuçta fiyat, günlük değişim ve borsa görünür. Ok tuşları + Enter ile seçilir, Esc kapatır.
- **Altta durum çubuğu:**
  - Grafiğin piyasasının bağlantısı, istek ağırlığı ve saat farkı.
  - Önbellekteki mum sayısı. Üzerine gelince önbellek yolu görünür.

## Proje sahibinin denemesi beklenenler

Adım 5 için (Claude ekrana tıklayamadığı için bunları proje sahibi dener):
- **Çizme:** her araçla çizmek. İki noktalılar tıkla–tıkla ya da basılı tutup sürükle. Kanal üç tık ister. Yatay ve dikey çizgi, not tek tık.
- **Düzenleme:** çizime tıklayıp seçmek, tutup taşımak, uçlarından (yuvarlak tutamaçlar) sürüklemek.
- **Seçili çizimin satırı:** renk, kalınlık, çizgi tipi, not metni, Fibonacci seviyeleri (boşlukla ayrılır, virgül de olur: `0 0,5 1`), kilitle, kopyala, sil.
- **Kısayollar:** Ctrl+Z / Ctrl+Y, Esc (aracı ya da seçimi bırakır), Delete.
- **Mıknatıs:** uçlar mumun açılış, en yüksek, en düşük ya da kapanışına yapışmalı.
- **Kayıt:** uygulamayı kapatıp açınca çizimler yerinde olmalı. Başka zaman diliminde de görünmeli. Spot ile vadelide ayrı tutulmalı.

Daha önceki adımlardan, hâlâ denenmemiş olanlar:
- İndikatör ekleme ve ayarları, panel kenarını sürükleme, şablonlar, Hacim düğmesi.
- Grafik tipleri, Log/% ölçekleri, uzun kaydırma, English düğmesi, internet kesilince toparlanma.

## Bilinen sınırlar

- İndikatörler uygulama kapanınca kaybolur. Kalıcı olmaları Adım 6'nın işi ("kaldığın yerden devam"). Şablonlar ve çizimler ise kalıcıdır.
- Lejantta ve çizim etiketlerinde yazı genişliği tahminle hesaplanıyor. Çok uzun etiketlerde aralıklar biraz geniş ya da dar olabilir.
- Dikdörtgenin sadece iki köşesi (çizilen noktalar) tutulup sürüklenebiliyor.
- Ölçüm aracı TradingView'daki geçici cetvel gibi değil, kalıcı bir çizim. İstenmezse silinir.
- Arayüz testleri `update` ve `view` fonksiyonları üzerinden yazıldı. Tıklama düzeyinde testler için `iced_test` gerekiyor, indirmek için onay bekleniyor.

## Açık sorular (proje sahibine sorulacak)

- `iced_test` (iced'in kendi test kütüphanesi, sadece testlerde kullanılır) indirilsin mi?
- Commit'lerde GitHub'ın gizli e-posta adresi (noreply) kullanılsın mı? (Daha önce önerildi, cevap bekliyor.)
- GitHub'da özel güvenlik bildirimi (private vulnerability reporting) açılsın mı? (Daha önce önerildi, cevap bekliyor.)

## Sıradaki adım: Adım 6, çalışma alanı

Henüz başlanmadı, proje sahibinin onayı bekleniyor. `docs/TASARIM.md` §4.5. Taslak:
- Çoklu grafik düzeni (iced `pane_grid`), grafikler arası eşleme.
- İzleme listesi: sağda açılıp kapanan panel.
- Kaldığın yerden devam: düzen, grafikler ve indikatörler `user.sqlite`'ta.
