<p align="center">
  <img src="assets/icon.svg" width="96" alt="Fitil">
</p>

<h1 align="center">Fitil</h1>

<p align="center">
  Açık kaynak, ücretsiz kripto grafik uygulaması<br>
  Open-source, free crypto charting app
</p>

<p align="center">
  <a href="#türkçe">Türkçe</a> · <a href="#english">English</a>
</p>

> **Geliştirme aşamasında.** Henüz indirilebilir bir sürüm yok.
> **Under development.** There is no downloadable release yet.

---

## Türkçe

Fitil, kripto para piyasaları için açık kaynak bir masaüstü grafik uygulamasıdır. Kendi bilgisayarında çalışır ve borsaya doğrudan bağlanır.

### Neden Fitil?

- **Sınır yok:** İndikatör, grafik ve çizim sayısında yapay bir sınır yok.
- **Sunucu yok, hesap yok:** Uygulama borsanın herkese açık verisine doğrudan bağlanır. API anahtarı ya da üyelik gerekmez.
- **Hafif ve hızlı:** Rust ile native olarak yazılır, grafikler ekran kartıyla çizilir.
- **Gizlilik:** Telemetri yok. Çizimlerin, düzenlerin ve ayarların sadece senin bilgisayarında durur.
- **Her yerde:** Windows, macOS ve Linux'ta çalışır. Arayüz Türkçe ve İngilizce, yeni diller eklenebilir.

Fitil bir analiz aracıdır. Al-sat ve emir gönderme özelliği yoktur, olmayacak.

### Durum

İlk sürüm (v0.1) geliştiriliyor. İlk sürümde Binance verisi, canlı mumlar, sınırsız indikatör, çizim araçları ve çoklu grafik olacak. Ayrıntılı plan için [tasarım belgesine](docs/TASARIM.md) bak.

### İndirme ve güvenlik

Sürümler hazır olduğunda Fitil **yalnızca** bu deponun [Releases](https://github.com/ylcnnemre/fitil/releases) sayfasından dağıtılacak. Her dosyanın yanında, değiştirilmediğini doğrulamak için bir SHA-256 özeti olacak.

Açık kaynak kripto uygulamalarının içine virüs eklenmiş kopyaları sık dolaşır. Fitil'i başka sitelerden indirme.

### Kaynaktan derleme

[Rust](https://rustup.rs) (stable) gerekir. Windows'ta Rust kurulumu sırasında Microsoft C++ derleme araçları da istenir.

```bash
git clone https://github.com/ylcnnemre/fitil
cd fitil
cargo run --release -p fitil-app
```

### Katkı

Katkılar memnuniyetle karşılanır. Başlamadan önce [CONTRIBUTING.md](CONTRIBUTING.md) dosyasını oku. Uygulamayı yeni bir dile çevirmek için Rust bilmek gerekmez.

### Yasal uyarı

- Fitil yatırım tavsiyesi vermez.
- Veriler doğrudan borsalardan alınır. Doğruluğu ve kesintisizliği garanti edilmez.
- Fitil'in Binance ya da başka bir borsayla resmi bir bağı yoktur.

### Lisans

[GPL-3.0](LICENSE). Fitil'i kullanabilir, değiştirebilir ve dağıtabilirsin. Değiştirilmiş bir sürümü dağıtırsan kaynak kodunu da aynı lisansla açman gerekir.

Simgeler [Lucide](https://lucide.dev) setinden gelir ([ISC lisansı](assets/fonts/LICENSE-lucide.txt)).

---

## English

Fitil is an open-source desktop charting app for crypto markets. It runs on your own computer and connects directly to the exchange.

### Why Fitil?

- **No limits:** No artificial cap on indicators, charts or drawings.
- **No server, no account:** The app reads the exchange's public market data directly. No API key or sign-up needed.
- **Light and fast:** Written natively in Rust, with charts rendered on the GPU.
- **Private:** No telemetry. Your drawings, layouts and settings stay on your computer.
- **Everywhere:** Runs on Windows, macOS and Linux. Turkish and English UI, with room for more languages.

Fitil is an analysis tool. It has no trading or order placement, and never will.

### Status

The first release (v0.1) is in development. It will cover Binance data, live candles, unlimited indicators, drawing tools and multi-chart layouts. The detailed plan is in the [design document](docs/TASARIM.md) (Turkish).

### Downloads and safety

Once releases are ready, Fitil will be distributed **only** through this repository's [Releases](https://github.com/ylcnnemre/fitil/releases) page. Each file will come with a SHA-256 checksum so you can check it has not been tampered with.

Malware-laced copies of open-source crypto apps are common. Do not download Fitil from anywhere else.

### Building from source

You need [Rust](https://rustup.rs) (stable). On Windows, the Rust installer also asks for the Microsoft C++ build tools.

```bash
git clone https://github.com/ylcnnemre/fitil
cd fitil
cargo run --release -p fitil-app
```

### Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) first. Translating the app into a new language does not require knowing Rust.

### Disclaimer

- Fitil does not give investment advice.
- Data comes directly from exchanges. Its accuracy and availability are not guaranteed.
- Fitil is not affiliated with Binance or any other exchange.

### License

[GPL-3.0](LICENSE). You may use, modify and distribute Fitil. If you distribute a modified version, you must release its source code under the same license.

The icons come from the [Lucide](https://lucide.dev) set ([ISC license](assets/fonts/LICENSE-lucide.txt)).
