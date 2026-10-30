# Fitil: Claude için talimatlar

Bu proje **Fitil**: Rust ile yazılan, açık kaynak (GPL-3.0) bir kripto grafik masaüstü uygulaması.

## Önce oku

- `memory/` klasörü projenin hafızasıdır. Oturum başında `memory/README.md` ile başla. Yapılanların ayrıntılı günlüğü `memory/03-gunluk.md` dosyasında.
- Ürünün tek doğru tanımı `docs/TASARIM.md` dosyasıdır. Kapsam değişecekse önce o belge güncellenir.

@memory/01-proje-ozeti.md
@memory/02-durum.md

## Proje sahibiyle çalışma

- **Türkçe konuş.** Proje sahibi Rust bilmiyor, kodun tamamını sen yazıyorsun. Teknik şeyleri sade anlat.
- **Onay al:** İndirme, kurulum, herkese açık yayın (push dışındaki repo ayarları, sürüm çıkarma) ve kapsam değişikliği gibi işlerden önce sor.
- **Kapsamı koru:** Al-sat, emir gönderme ve API anahtarı gerektiren özellikler **önerilmez**. Bu bir proje kararıdır.
- **Hafızayı güncelle:** Anlamlı bir işin sonunda `memory/02-durum.md` ve `memory/03-gunluk.md` dosyalarını güncelle.

## Kod kuralları

- Kod, kod içi açıklamalar ve commit mesajları İngilizce.
- Kullanıcıya görünen her metin `locales/` altındaki Fluent dosyalarından gelir. Kodun içine Türkçe veya İngilizce metin gömülmez.
- Her değişiklikten sonra şunlar temiz geçmeli:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
