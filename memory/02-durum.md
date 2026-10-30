# Şu Anki Durum

**Son güncelleme:** 30 Ekim 2026

## Neredeyiz?

- **Tasarım:** `docs/TASARIM.md` onaylı.
- **Adım 1 tamamlandı:** iskelet, GitHub deposu, CI.
- **Adım 2 tamamlandı:** Binance bağlantısı ve yerel depo. Ayrıntılar `03-gunluk.md` dosyasında.

Uygulama şu an şunları gösteriyor:
- Solda canlı fiyat listesi: Spot ve Vadeli sekmeleri, arama, dolar hacmine göre ilk 100 sembol.
- Sağda seçilen sembolün ayrıntıları:
  - zaman dilimi seçici (özel zaman dilimleri dahil),
  - açık mum ve kapanışa kalan süre,
  - önbellek bilgisi ve "Daha eski mumları yükle" düğmesi.
- Altta durum çubuğu: bağlantı durumu, istek ağırlığı, saat farkı, önbellek yolu, sürüm.

## Proje sahibinin denemesi beklenenler

Bunları Claude tıklayamadığı için deneyemedi:
- Zaman dilimini 45m gibi özel bir değere çevirmek.
- Vadeli sekmesi.
- Arama kutusu.
- English düğmesi.
- İnterneti kesip geri açınca durum çubuğunun "Bağlantı koptu…" deyip toparlanması.

Veri katmanı ise gerçek Binance'e karşı testlerle doğrulandı.

## Sıradaki adım: Adım 3, grafik motoru

Henüz başlanmadı, proje sahibinin onayı bekleniyor. Taslak plan:
- `fitil-chart` crate'i:
  - iced Canvas üzerinde mum çizimi,
  - fiyat ve zaman eksenleri,
  - fare tekerleğiyle yakınlaştırma, sürükleyerek kaydırma, eksen sürükleyerek ölçekleme,
  - artı imleci ve OHLC bilgisi,
  - son fiyat çizgisi ve geri sayım.
- Grafik sola kaydırıldıkça `data::load_older` ile otomatik geçmiş yükleme. Şu an bu iş bir düğmeyle yapılıyor.
- Ayrıntı panelindeki metin bilgilerinin yerini grafik alacak.
- Özel zaman dilimlerinde baştaki eksik mum atlanacak: `aggregate` ilk ve son kovayı yarım üretebiliyor.
