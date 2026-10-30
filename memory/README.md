# Proje Hafızası

Bu klasör, Fitil projesinde bugüne kadar konuşulanları, kararlaştırılanları ve yapılanları kayıt altında tutar. Amaç, yeni bir Claude oturumu açıldığında ya da projeye yeni biri katıldığında hiçbir şeyin unutulmamasıdır.

## Okuma sırası

| Dosya | İçerik |
|---|---|
| [01-proje-ozeti.md](01-proje-ozeti.md) | Fitil nedir, verilen kararlar ve nedenleri, proje sahibiyle çalışma şekli |
| [02-durum.md](02-durum.md) | Şu an neredeyiz, sıradaki adım ne |
| [03-gunluk.md](03-gunluk.md) | Tarih sırasıyla yapılan her şey (oturum günlüğü) |
| [04-teknik-notlar.md](04-teknik-notlar.md) | Geliştirme ortamı, komutlar, dikkat edilecek noktalar |

Ürünün ayrıntılı tanımı için tek doğru kaynak [docs/TASARIM.md](../docs/TASARIM.md) dosyasıdır. Bu klasör o belgenin yerini tutmaz. Burada kararların hikâyesi ve işin nerede olduğu anlatılır.

## Kurallar

- **Oturum başında:** Claude bu klasörü okur. Proje kökündeki `CLAUDE.md` bunu otomatik hatırlatır, `01` ve `02` dosyalarını da otomatik yükler.
- **Anlamlı bir işin sonunda:** `02-durum.md` ve `03-gunluk.md` güncellenir.
- **Karar değişirse:** Önce `docs/TASARIM.md`, sonra `01-proje-ozeti.md` güncellenir.
- **Gizli bilgi yazılmaz:** Bu klasör herkese açık GitHub deposunda durur. Şifre, token, e-posta adresi gibi bilgiler buraya yazılmaz.
