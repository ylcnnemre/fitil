# Şu Anki Durum

**Son güncelleme:** 2 Kasım 2026

## Neredeyiz?

- **Tasarım:** `docs/TASARIM.md` onaylı. Mimari bölümü (§6) 31 Ekim'de yeniden yazıldı. Adım 4 ile tablo ve §6.6 (veri dosyaları), TradingView düzeniyle §4.5, Adım 5 ile §4.4 ve §6.5 (simgeler) güncellendi. §4.4 yeni çizgi araçları ve açılır menüyle (karar 19), sonra düzenleme penceresi, Fibonacci ve Gann gruplarıyla (karar 20) yeniden güncellendi.
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
- **Çizim çubuğu TradingView düzeninde (karar 19), kodlandı, proje sahibinin denemesi bekleniyor:**
  - Çizgiler, kanallar ve dirgenler tek düğmede; tıklayınca sağda menü açılıyor (`flyout.rs`, `tools.rs`).
  - 10 yeni araç: bilgi çizgisi, trend açısı, kesişim çizgisi, regresyon trendi, düz tepe/dip, ayrık kanal, dirgen, Schiff, modifiye Schiff, iç dirgen.
  - Alt+T/H/J/V/C kısayolları.
  - Kesikli ve noktalı çizgilerdeki fazladan çizgi hatası düzeltildi.
- **Kaydırma akıcılığı tamamlandı (2 Kasım 2026):** proje sahibi release sürümde denedi ve "Evet, akıcı" dedi.
  - Mumlar, hacim çubukları, OHLC çubukları, histogram sütunları ve noktalar tek tek çiziliyor. Yaklaşık 560 mumda bir kare release'te 2,3 ms yerine 0,4 ms sürüyor.
  - Proje sahibine artık release sürüm açılıyor (`target\release\fitil.exe`). Debug sürüm çizimde yaklaşık 10 kat yavaş, takılmanın ana nedeni oydu.
- **Çizimlerin düzenleme penceresi (karar 20), kodlandı, proje sahibinin denemesi bekleniyor (2 Kasım 2026):**
  - Çizime çift tıklayınca ya da satırdaki ayarlar düğmesiyle açılıyor (`drawing_dialog.rs`). Stil, Koordinatlar, Görünürlük sekmeleri; Şablon menüsü, İptal, Tamam.
  - Fib düzeltmesi TradingView gibi: 11 seviye kendi renginde, 24 seviyeye kadar; tek renk, trend çizgisi, uzatma, arka plan, karşıt, etiketler. Alt+F.
  - Görünürlük: zaman birimine göre aç/kapa ve aralık. Şablonlar ve varsayılan görünüm `user.sqlite`'ta (göç 3).
  - Çizimlerin kayıt biçimi 2. sürüm; eski kayıtlar okunuyor ve görünüşü korunuyor.
  - Sırada Fibonacci ve Gann gruplarının 14 yeni aracı (proje sahibi pencereyi denedikten sonra).

Uygulama şu an şunları gösteriyor:
- **Üstte araç çubuğu:**
  - Sembol adı (büyüteç simgesiyle). Tıklayınca sembol arama penceresi açılır. Grafikte yazmaya başlamak da açar.
  - Sık kullanılan zaman dilimleri (1m 5m 15m 1h 4h 1d 1w) ve "Diğer" listesi.
  - Grafik tipi, Log, %, **İndikatörler**, **Hacim**, geri al ve yinele simgeleri.
  - En sağda dil düğmeleri.
- **Solda çizim araçları çubuğu:**
  - Seç ve taşı.
  - Çizgi araçları düğmesi: en son seçilen aracın simgesini gösterir, köşesinde küçük bir üçgen var. Tıklayınca sağda menü açılır:
    - ÇİZGİLER: trend çizgisi (Alt+T), ışın, bilgi çizgisi, uzatılmış çizgi, trend açısı, yatay çizgi (Alt+H), yatay ışın (Alt+J), dikey çizgi (Alt+V), kesişim çizgisi (Alt+C).
    - KANALLAR: paralel kanal, regresyon trendi, düz tepe/dip, ayrık kanal.
    - DİRGENLER: dirgen, Schiff dirgeni, modifiye Schiff dirgeni, iç dirgen.
  - Dikdörtgen, Fibonacci (Alt+F).
  - Metin notu, ölçüm.
  - Mıknatıs, tümünü kilitle, çizimleri gizle, tümünü sil (iki tıkla).
- **Grafik ekranın geri kalanını kaplar.** Açılışta BTCUSDT açılır.
  - Araç çubuğunun altında indikatör etiketleri.
  - Bir çizim seçiliyken onun satırı da çıkar: renk, kalınlık, çizgi tipi, not metni, ayarlar, kilitle, kopyala, sil.
  - Çizime çift tıklayınca ayarlar penceresi açılır (Stil, Koordinatlar, Görünürlük, Şablon).
  - Yatay çizgilerin fiyatı fiyat ekseninde, dikey çizgilerin tarihi zaman ekseninde görünür. Kesişim çizgisinde ikisi de.
- **Sembol arama penceresi:**
  - Tümü / Spot / Vadeli sekmeleri.
  - Sonuçlar önce yakınlığa, sonra dolar hacmine göre sıralanır.
  - Her sonuçta fiyat, günlük değişim ve borsa görünür. Ok tuşları + Enter ile seçilir, Esc kapatır.
- **Altta durum çubuğu:**
  - Grafiğin piyasasının bağlantısı, istek ağırlığı ve saat farkı.
  - Önbellekteki mum sayısı. Üzerine gelince önbellek yolu görünür.

## Proje sahibinin denemesi beklenenler

Düzenleme penceresi için (release sürümde):
- **Açma:** çizime çift tıklamak; seçili çizimin satırındaki ayarlar (dişli) düğmesi.
- **Stil:** renk seçici (kareye tıklayınca palet açılır), kalınlık, çizgi tipi, arka plan ve kaydırıcısı. Fib'de seviye tablosu (aç/kapa, değer yazma, renk), tek renk, trend çizgisi, uzat, karşıt, fiyatlar, seviyeler değer/yüzde, etiket yerleri.
- **Koordinatlar:** fiyatı ve tarihi elle yazmak (`2026-11-02 14:30` biçiminde).
- **Görünürlük:** bir birimi kapatınca o zaman dilimlerinde çizimin kaybolması; aralık yazmak (ör. saat 1–4).
- **İptal / Esc / dışarı tıklamak** her şeyi geri almalı; **Tamam** tutmalı ve Ctrl+Z tek adımda geri almalı.
- **Şablon:** isimle kaydetmek, seçince uygulanması, silmek; "Varsayılan olarak kaydet" sonrası yeni çizimlerin o görünümle başlaması; "Varsayılana dön", "Fitil'in görünümü".
- **Eski çizimler:** önceden çizilmiş Fibonacci'lerin aynı görünmesi (tek renk açık gelir).

Açılır menü ve yeni araçlar için (Claude ekrana tıklayamadığı için bunları proje sahibi dener):
- **Menü:** sol çubuktaki çizgi düğmesine tıklayınca menünün açılması; menüden seçince kapanması; dışarı tıklayınca, Esc'ye basınca ya da düğmeye yeniden tıklayınca kapanması. Seçilen aracın simgesinin düğmede kalması.
- **Yeni araçlarla çizmek:** bilgi çizgisi ve trend açısı (iki tık), kesişim çizgisi (tek tık), regresyon (iki tık; sadece zamanlar önemli), düz tepe/dip, ayrık kanal ve dirgenler (üç tık).
- **Kısayollar:** Alt+T, Alt+H, Alt+J, Alt+V, Alt+C.
- **Kesikli/noktalı çizgi:** bir kanalda ya da Fibonacci'de çizgi tipini değiştirince fazladan çapraz çizgi çıkmamalı.

Adım 5 için:
- **Çizme:** her araçla çizmek. İki noktalılar tıkla–tıkla ya da basılı tutup sürükle. Kanal üç tık ister. Yatay ve dikey çizgi, not tek tık.
- **Düzenleme:** çizime tıklayıp seçmek, tutup taşımak, uçlarından (yuvarlak tutamaçlar) sürüklemek.
- **Seçili çizimin satırı:** renk, kalınlık, çizgi tipi, not metni, ayarlar, kilitle, kopyala, sil. (Fibonacci seviyeleri artık düzenleme penceresinde.)
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
- Dirgenlerde TradingView'daki ara seviyeler (0.5 gibi) ve çizgileri uzatma seçenekleri yok. Regresyonda sapma sayısı ve kaynak değiştirilemiyor (±2, kapanış).
- Grup düğmesinin hatırladığı son araç uygulama kapanınca unutulur. Kalıcı olması Adım 6'nın işi.
- Düzenleme penceresinin renk seçicisi 12 renklik sabit paletten seçer. TradingView'daki serbest renk ve renk saydamlığı seçicisi yok.
- Pencere, TradingView'ın araca özel bütün seçeneklerini taşımıyor: trend çizgisinin uzatma ve istatistikleri, dikdörtgenin metni, regresyonun sapma ayarı gibi.
- Koordinatlar tarih ve saat olarak yazılır (TradingView çubuk numarası kullanır).
- Arayüz testleri `update` ve `view` fonksiyonları üzerinden yazıldı. Tıklama düzeyinde testler için `iced_test` gerekiyor, indirmek için onay bekleniyor.

## Açık sorular (proje sahibine sorulacak)

- Grafiği dikey çekerken ilk 12 pikselin yok sayılması kaldırılsın mı? TradingView'da böyle bir bekleme yoksa kaldırılacak (2 Kasım'da soruldu).
- `iced_test` (iced'in kendi test kütüphanesi, sadece testlerde kullanılır) indirilsin mi?
- Commit'lerde GitHub'ın gizli e-posta adresi (noreply) kullanılsın mı? (Daha önce önerildi, cevap bekliyor.)
- GitHub'da özel güvenlik bildirimi (private vulnerability reporting) açılsın mı? (Daha önce önerildi, cevap bekliyor.)

## Sıradaki adım

Proje sahibi düzenleme penceresini deneyince Fibonacci ve Gann gruplarının 14 yeni aracı yapılacak (karar 20, liste `docs/TASARIM.md` §4.4'te): trend temelli fib uzatması, Fib kanalı, Fib zaman bölgesi, Fib hız direnç fanı, trend temelli fib zamanı, Fib çemberleri, Fib sarmalı, Fib hız direnç yayları, Fib takozu, Andrews çatalı (Pitchfan); Gann kutusu, sabit Gann karesi, Gann karesi, Gann fanı. Sol çubuktaki Fibonacci düğmesi grup düğmesine dönüşecek (FİBONACCİ ve GANN bölümleri). Her aracın tanımı TradingView'ın yardım sayfalarından alınıp §4.4'e yazılacak.

Sonra sol çubuğun diğer grupları (şekiller, notlar, ölçüm ve tahmin araçları); hangi araçların ekleneceği proje sahibiyle konuşulacak.

Ardından Adım 6, çalışma alanı. Henüz başlanmadı, proje sahibinin onayı bekleniyor. `docs/TASARIM.md` §4.5. Taslak:
- Çoklu grafik düzeni (iced `pane_grid`), grafikler arası eşleme.
- İzleme listesi: sağda açılıp kapanan panel.
- Kaldığın yerden devam: düzen, grafikler ve indikatörler `user.sqlite`'ta.
