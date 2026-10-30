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
- **Adım 1 başladı:** Microsoft C++ derleme araçları winget ile kuruluyor (Visual Studio Build Tools 2022, VCTools iş yükü).
