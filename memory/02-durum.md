# Şu Anki Durum

**Son güncelleme:** 31 Ekim 2026

## Neredeyiz?

- **Tasarım:** `docs/TASARIM.md` onaylı.
- **Adım 1 tamamlandı:** iskelet, GitHub deposu, CI.
- **Adım 2 tamamlandı:** Binance bağlantısı ve yerel önbellek.
- **Adım 3 tamamlandı:** grafik motoru. Ayrıntılar `03-gunluk.md` dosyasında.

Uygulama şu an şunları gösteriyor:
- **Solda canlı fiyat listesi:** Spot ve Vadeli sekmeleri, arama, dolar hacmine göre ilk 100 sembol.
- **Sağda seçilen sembolün grafiği:**
  - Araç çubuğu: zaman dilimi (özel olanlar dahil), grafik tipi (Mum, Çubuk, Çizgi, Alan, Heikin Ashi), Log ve % düğmeleri, önbellek bilgisi.
  - Grafik: hacim, ızgara, fiyat ve zaman eksenleri, artı imleci, gösterge (lejant), son fiyat ve geri sayım.
  - Etkileşim: tekerlekle yakınlaştırma, sürükleyerek kaydırma, eksenleri sürükleyerek ölçekleme, çift tıklayarak sıfırlama.
  - Sola kaydırınca eski mumlar kendiliğinden yükleniyor.
- **Altta durum çubuğu.**
- **Komut satırı:** `fitil --open spot:BTCUSDT:1h` grafiği doğrudan açar. Claude test için bunu kullanıyor.

## Proje sahibinin denemesi beklenenler

- Grafik tiplerini ve Log/% ölçeklerini denemek.
- Sola doğru uzun kaydırıp geçmişin yüklenmesini izlemek.
- 1d / 1w gibi uzun zaman dilimlerinde ay ve yıl etiketlerine bakmak.
- Adım 2'den kalanlar: English düğmesi, arama kutusu, internet kesilince toparlanma.

Proje sahibi Adım 3 sırasında grafiği zaten kullandı (ETHUSDT 45m): yakınlaştırdı, kaydırdı ve eski mumlar kendiliğinden yüklendi.

## Sıradaki adım: Adım 4, indikatörler ve paneller

Henüz başlanmadı, proje sahibinin onayı bekleniyor. Taslak plan:
- **`fitil-indicators` crate'i:** `docs/TASARIM.md` §4.3'teki indikatörler. Artımlı hesaplanacaklar ve bilinen referans değerlerle test edilecekler.
- **Grafik:**
  - Ana grafiğin üstüne çizilen indikatörler (MA, Bollinger, VWAP, SuperTrend, Ichimoku…).
  - Alt paneller (RSI, MACD, Stokastik, ATR, OBV, CVD…), panel yüksekliği sürüklenebilir.
  - Hacim de ayrı panele alınabilecek.
- **Arayüz:** İndikatör ekleme penceresi (arama ile) ve ayar penceresi (periyot, kaynak, renk, kalınlık, görünürlük).
- **İndikatör şablonları** (§4.3).
- **Grafik ayarları:** Grafik tipi ve ölçek şu an sadece uygulama açıkken hatırlanıyor. Kalıcı kayıt Adım 6'da.
