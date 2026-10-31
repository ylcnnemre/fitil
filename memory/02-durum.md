# Şu Anki Durum

**Son güncelleme:** 31 Ekim 2026

## Neredeyiz?

- **Tasarım:** `docs/TASARIM.md` onaylı. Mimari bölümü (§6) 31 Ekim'de yeniden yazıldı. Adım 4 ile tablo ve §6.6 (veri dosyaları) güncellendi.
- **Adım 1 tamamlandı:** iskelet, GitHub deposu, CI.
- **Adım 2 tamamlandı:** Binance bağlantısı ve yerel önbellek.
- **Adım 3 tamamlandı:** grafik motoru.
- **Mimari düzenleme tamamlandı:** çok borsalı, port/adaptör düzeni.
- **Adım 4 kodlandı, proje sahibinin denemesi bekleniyor:** indikatörler ve paneller.
  - `fitil-indicators`: 22 indikatör, artımlı hesap, referans değerlerle testler.
  - `fitil-store`: mum önbelleğinden ayrı `user.sqlite` (şablonlar).
  - `fitil-market`: grafikteki indikatörler (`Studies`) ve şablon kaydetme/okuma.
  - `fitil-chart`: indikatör çizimi, alt paneller, sürüklenebilir panel kenarı.
  - `fitil-app`: İndikatörler penceresi, ayar penceresi, indikatör etiketleri, Hacim düğmesi.
  - Ayrıntılar `03-gunluk.md` dosyasında.

Uygulama şu an şunları gösteriyor (31 Ekim'de TradingView düzenine geçildi, karar 17):
- **Üstte araç çubuğu:**
  - Sembol adı. Tıklayınca sembol arama penceresi açılır. Grafikte yazmaya başlamak da açar.
  - Sık kullanılan zaman dilimleri (1m 5m 15m 1h 4h 1d 1w) ve "Diğer" listesi.
  - Grafik tipi, Log, %, **İndikatörler**, **Hacim** (mum altı hacim çubukları).
  - En sağda dil düğmeleri.
- **Grafik ekranın tamamını kaplar.** Solda sembol listesi yok. Açılışta en çok işlem gören Bitcoin paritesi (BTCUSDT) açılır.
  - Araç çubuğunun altında her indikatör için bir etiket: adına tıklayınca ayarları açılır, ●/○ gizler/gösterir, × kaldırır.
  - Fiyatın üstünde indikatörler, altında paneller (RSI, MACD, Hacim…), otomatik geçmiş yükleme.
- **Sembol arama penceresi:**
  - Tümü / Spot / Vadeli sekmeleri.
  - Sonuçlar önce yakınlığa (tam ad, aynı coin, baştan eşleşme), sonra dolar hacmine göre sıralanır.
  - Her sonuçta fiyat, günlük değişim ve borsa görünür. Ok tuşları + Enter ile seçilir, Esc kapatır.
- **Altta durum çubuğu:**
  - Grafiğin piyasasının bağlantısı, istek ağırlığı ve saat farkı. Ağırlık ve saat farkı yalnızca bunları bildiren borsalarda görünür.
  - Önbellekteki mum sayısı. Üzerine gelince önbellek yolu görünür.

## Proje sahibinin denemesi beklenenler

Yeni düzen için:
- Sembol arama: sembol adına tıklama, grafikte yazmaya başlama, sekmeler, ok tuşları + Enter. (Proje sahibi test penceresinde aramayı kullanıp ETHUSDT vadeliyi açtı; çalıştığı görüldü.)

Adım 4 için (Claude ekrana tıklayamadığı için bunları proje sahibi dener):
- İndikatörler düğmesi → listeden ekleme, arama ("rsi", "bant").
- Bir indikatörün etiketine tıklayıp ayar değiştirme: uzunluk yazma (virgülle de olur: `2,5`), kaynak, renk, kalınlık, çizgiyi gizleme, Varsayılanlar.
- Panel kenarını sürükleyerek yükseklik ayarlama. Sembol değişince panel yükseklikleri korunmalı.
- Şablon: isim yazıp Kaydet, başka bir sembolde Uygula, Sil (iki tıkla: "Emin misin?").
- Esc ile pencereyi kapatma, pencerenin dışına tıklayınca kapanma.
- Hacim düğmesiyle mum altı hacmi kapatıp açma.

Önceki adımlardan:
- Grafik tipleri, Log/% ölçekleri, uzun kaydırma, 1d/1w etiketleri.
- English düğmesi, arama, internet kesilince toparlanma.

## Bilinen sınırlar

- İndikatörler uygulama kapanınca kaybolur. Kalıcı olmaları Adım 6'nın işi ("kaldığın yerden devam"). Şablonlar ise kalıcıdır.
- Lejantta yazı genişliği tahminle hesaplanıyor. Çok uzun etiketlerde aralıklar biraz geniş ya da dar olabilir.
- Arayüz testleri şimdilik `update` ve `view` fonksiyonları üzerinden yazıldı. Tıklama düzeyinde testler için `iced_test` kütüphanesi gerekiyor. İndirmek için proje sahibinin onayı istenecek.

## Açık sorular (proje sahibine sorulacak)

- `iced_test` (iced'in kendi test kütüphanesi, sadece testlerde kullanılır) indirilsin mi?
- Commit'lerde GitHub'ın gizli e-posta adresi (noreply) kullanılsın mı? (Daha önce önerildi, cevap bekliyor.)
- GitHub'da özel güvenlik bildirimi (private vulnerability reporting) açılsın mı? (Daha önce önerildi, cevap bekliyor.)

## Sıradaki adım: Adım 5, çizim araçları

Henüz başlanmadı, proje sahibinin onayı bekleniyor. `docs/TASARIM.md` §4.4'teki araçlar. Taslak:
- Çizimler `user.sqlite`'ta sembol başına saklanır (yeni göç).
- Çizim mantığı (tutamaçlar, mıknatıs, seçim) `fitil-chart`'ta, kayıt `fitil-market` ve `fitil-store`'da.
