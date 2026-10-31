# Şu Anki Durum

**Son güncelleme:** 31 Ekim 2026

## Neredeyiz?

- **Tasarım:** `docs/TASARIM.md` onaylı. Mimari bölümü (§6) 31 Ekim'de yeniden yazıldı.
- **Adım 1 tamamlandı:** iskelet, GitHub deposu, CI.
- **Adım 2 tamamlandı:** Binance bağlantısı ve yerel önbellek.
- **Adım 3 tamamlandı:** grafik motoru.
- **Mimari düzenleme tamamlandı.** Bu bir ara adım: Adım 4'ten önce, proje sahibinin isteğiyle yapıldı.
  - Proje çok borsalı, port/adaptör düzenine geçti.
  - Crate'ler: `fitil-core`, `fitil-exchange` (arayüz), `fitil-binance` (adaptör), `fitil-store`, `fitil-market` (uygulama katmanı), `fitil-indicators`, `fitil-chart`, `fitil-app`.
  - Önbellek göç 2 ile yeni anahtarlara taşındı. Proje sahibinin gerçek önbelleğinin bir kopyasıyla doğrulandı.
  - Ayrıntılar `03-gunluk.md` dosyasında.

Uygulama şu an şunları gösteriyor:
- **Üst çubuk:** borsa seçici (şimdilik sadece Binance), Spot ve Vadeli düğmeleri, arama, dil düğmeleri.
- **Solda canlı fiyat listesi.**
- **Sağda grafik:** zaman dilimi, grafik tipi, Log ve %, otomatik geçmiş yükleme.
- **Altta durum çubuğu:** İstek ağırlığı ve saat farkı yalnızca bunları bildiren borsalarda görünür.

## Proje sahibinin denemesi beklenenler

- Grafik tipleri, Log/% ölçekleri, uzun kaydırma, 1d/1w etiketleri.
- English düğmesi, arama, internet kesilince toparlanma.
- **Önemli:** Proje sahibinin bilgisayarında hâlâ mimari öncesi derlemeden açılmış bir Fitil penceresi var.
  - Yeni sürüm ilk açıldığında önbelleği göç 2 ile taşıyacak.
  - Eski pencere açıkken yeni sürümü açmak eski pencerede önbellek hatasına yol açar. Önce eski pencere kapatılmalı.

## Sıradaki adım: Adım 4, indikatörler ve paneller

Henüz başlanmadı, proje sahibinin onayı bekleniyor. Taslak plan:
- **`fitil-indicators`:**
  - `docs/TASARIM.md` §4.3'teki indikatörler.
  - Saf hesap: mumlar girer, seriler çıkar. Artımlı hesaplanır, bilinen referans değerlerle test edilir.
  - Alış/Satış Hacmi ve CVD, `Exchange::features().taker_volume` false olan borsalarda gizlenir.
- **Grafik:**
  - Ana grafiğin üstüne çizilen indikatörler.
  - Alt paneller, yükseklikleri sürüklenebilir.
  - Hacim ayrı bir panele alınabilecek.
- **Arayüz:** indikatör ekleme penceresi (arama ile), ayar penceresi, indikatör şablonları.
- **Mimari kural:** Hesap `fitil-indicators`'ta, hangi indikatörlerin açık olduğu bilgisi uygulama katmanında, çizim `fitil-chart`'ta.
