# Şu Anki Durum

**Son güncelleme:** 30 Ekim 2026

## Neredeyiz?

- **Tasarım:** `docs/TASARIM.md` proje sahibi tarafından onaylandı.
- **Faz 1, Adım 1 tamamlandı:** geliştirme ortamı, proje iskeleti, GitHub deposu, otomatik derleme.
  - Depo: https://github.com/ylcnnemre/fitil (herkese açık)
  - İlk CI çalışması Windows, macOS ve Linux'ta yeşil. Format, clippy, testler ve cargo-deny geçiyor.
  - Uygulama şu an boş bir pencere gösteriyor:
    - İlk açılışta sistem dilini seçiyor (Türkçe ya da İngilizce), Türkçe/English düğmeleriyle dil değişiyor.
    - Koyu tema, sürüm numarası görünüyor.

## Sıradaki adım: Adım 2, Binance bağlantısı ve yerel depo

Proje sahibine anlatıldı, başlamak için onayı bekleniyor. Plan:

1. **`fitil-core`:** Mum (OHLCV, işlem sayısı, taker alış hacmi), sembol (piyasa, fiyat hassasiyeti) ve zaman dilimi tipleri. Özel zaman dilimleri, ona tam bölünen en büyük hazır zaman diliminden üretilir.
2. **`fitil-exchange`, Binance adaptörü** (Spot ve USDT-M):
   - sembol listesi (exchangeInfo),
   - geçmiş mumlar 1000'erli sayfalarla,
   - WebSocket ile açık mum ve tüm sembollerin fiyatları,
   - kopunca yeniden bağlanma ve boşluk doldurma,
   - istek limiti takibi (`X-MBX-USED-WEIGHT-1M`),
   - sunucu saati farkı,
   - 451 erişim engeli için anlaşılır hata.
3. **`fitil-store`:** SQLite mum önbelleği, uygulama klasöründe. İnen mum tekrar indirilmez.
4. **Testler:** Kaydedilmiş gerçek Binance yanıtlarıyla. İnternete bağlı değil, CI'da çalışır.
5. **Görünen sonuç:**
   - Canlı fiyat listesi: sembol, fiyat, 24 saatlik değişim yüzdesi.
   - Bir sembole tıklanınca geçmiş mumlar indirilir ve özet gösterilir: mum sayısı, en eski tarih. İkinci tıklamada veri önbellekten gelir.
   - İnternet kesilip gelince otomatik toparlanma.

**Teknik not:** TLS için `rustls` kullanılacak, kripto sağlayıcısı `ring` olacak. Varsayılan aws-lc'nin lisansları GPL ile sorun çıkarabilir, cargo-deny ile kontrol edilmeli.
