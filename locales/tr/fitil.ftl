# Fitil — Türkçe arayüz metinleri.
# Her anahtar en/fitil.ftl dosyasında da bulunmalıdır.

app-title = Fitil
app-version = Sürüm { $version }

language-tr = Türkçe
language-en = English

market-spot = Spot
market-futures = Vadeli (USDⓈ-M)
market-tag-spot = SPOT
market-tag-futures = VADELİ

## Sembol arama penceresi

search-title = Sembol ara
search-placeholder = Sembol ya da coin adı yaz (ör. BTC, ETHUSDT)…
search-all = Tümü
search-empty = Eşleşen sembol yok.
search-more = { $total } sonuçtan ilk { $shown } tanesi gösteriliyor. Biraz daha yazarak aramayı daraltabilirsin.
search-market-loading = { $market } sembolleri yükleniyor…
symbols-loading = Semboller yükleniyor…
symbols-retry = Tekrar dene

detail-source = { $source } mumlardan üretiliyor
detail-cached = Önbellekte { $count } mum ({ $timeframe })
detail-complete = tüm geçmiş yüklendi
detail-loading = Yükleniyor…

chart-kind-candles = Mum
chart-kind-bars = Çubuk (OHLC)
chart-kind-line = Çizgi
chart-kind-area = Alan
chart-kind-heikin-ashi = Heikin Ashi
scale-log = Log
scale-percent = %
chart-hint = Yaz: sembol ara · Tekerlek: yakınlaştır · Sürükle: kaydır · Fiyat eksenini sürükle: dikey ölçek · Zaman eksenini sürükle: yatay ölçek · Panel kenarını sürükle: panel yüksekliği · Çift tıkla: sıfırla
chart-indicators = İndikatörler
chart-volume-toggle = Hacim
chart-symbol-search = Sembol ara
chart-symbol-hint = Sembolü değiştir. Grafiğin üzerinde yazmaya başlaman da yeter.
chart-pick-symbol = Grafiğini görmek istediğin sembolü ara.
chart-more-timeframes = Diğer
chart-undo = Geri al (Ctrl+Z)
chart-redo = Yinele (Ctrl+Y)
chart-note = Metin
chart-bars = mum
chart-hour-suffix = s
chart-minute-suffix = dk

## Çizim araçları (grafiğin solundaki çubuk ve seçili çizimin satırı)

drawing-cursor = Seç ve taşı
drawing-trend-line = Trend çizgisi
drawing-ray = Işın
drawing-extended-line = Uzatılmış çizgi
drawing-horizontal-line = Yatay çizgi
drawing-horizontal-ray = Yatay ışın
drawing-vertical-line = Dikey çizgi
drawing-rectangle = Dikdörtgen
drawing-parallel-channel = Paralel kanal
drawing-fib-retracement = Fibonacci düzeltmesi
drawing-text = Metin notu
drawing-measure = Ölçüm
drawing-magnet = Mıknatıs: noktalar mumun açılış, en yüksek, en düşük ya da kapanışına yapışır
drawing-lock-all = Tüm çizimleri kilitle
drawing-unlock-all = Çizimlerin kilidini aç
drawing-hide-all = Çizimleri gizle
drawing-show-all = Çizimleri göster
drawing-clear-all = Bu sembolün tüm çizimlerini sil
drawing-clear-confirm = Emin misin? Silmek için tekrar tıkla.
drawing-line-solid = Düz
drawing-line-dashed = Kesikli
drawing-line-dotted = Noktalı
drawing-lock = Kilitle
drawing-unlock = Kilidi aç
drawing-copy = Kopyala
drawing-delete = Sil (Delete)
drawing-levels = Seviyeler
drawing-text-placeholder = Not yaz…
chart-open = A
chart-high = Y
chart-low = D
chart-close = K
chart-volume = Hac
chart-months = Oca, Şub, Mar, Nis, May, Haz, Tem, Ağu, Eyl, Eki, Kas, Ara
chart-magnitudes = B, Mn, Mr
chart-day-suffix = g
chart-loading-older = Daha eski mumlar yükleniyor…

status-connecting = Canlı veriye bağlanıyor…
status-connected = Canlı veri bağlı
status-reconnecting = Bağlantı koptu, { $seconds } sn sonra yeniden denenecek ({ $attempt }. deneme)
status-weight = İstek ağırlığı: { $used } / { $limit }
status-clock = Saat farkı: { $seconds } sn
status-cache = Önbellek: { $path }

error-restricted = Bu borsa bulunduğun yerden erişime izin vermiyor.
error-rate-limited = Borsa çok fazla istek gönderildiğini bildirdi. { $seconds } sn bekleyip tekrar dene.
error-network = Borsaya ulaşılamadı. İnternet bağlantını kontrol et.
error-exchange = Borsa bir hata döndürdü: { $detail }
error-storage = Yerel önbellek kullanılamıyor: { $detail }
error-user-data = Kayıtlı verilerin (şablonlar) kullanılamıyor: { $detail }

## İndikatör penceresi ve şablonlar

indicators-title = İndikatörler
indicators-search = İndikatör ara…
indicators-empty = Eşleşen indikatör yok.
templates-title = Şablonlar
templates-hint = Grafikteki indikatörleri ayarlarıyla birlikte bir isimle kaydet, sonra tek tıkla başka bir grafiğe uygula.
templates-name = Şablon adı
templates-save = Kaydet
templates-apply = Uygula
templates-delete = Sil
templates-confirm-delete = Emin misin?
templates-empty = Henüz kayıtlı şablon yok.

## Bir indikatörün ayarları

study-settings-title = { $name } ayarları
study-inputs = Girdiler
study-style = Görünüm
study-width = Kalınlık
study-show = Göster
study-hide = Gizle
study-remove = Kaldır
study-defaults = Varsayılanlar
study-done = Tamam

## İndikatörler: uzun ad (listede) ve kısa ad (grafikte)

indicator-sma = Basit Hareketli Ortalama (SMA)
indicator-sma-short = SMA
indicator-ema = Üstel Hareketli Ortalama (EMA)
indicator-ema-short = EMA
indicator-wma = Ağırlıklı Hareketli Ortalama (WMA)
indicator-wma-short = WMA
indicator-vwma = Hacim Ağırlıklı Hareketli Ortalama (VWMA)
indicator-vwma-short = VWMA
indicator-bollinger = Bollinger Bantları
indicator-bollinger-short = BB
indicator-keltner = Keltner Kanalı
indicator-keltner-short = KC
indicator-donchian = Donchian Kanalı
indicator-donchian-short = DC
indicator-supertrend = SuperTrend
indicator-supertrend-short = SuperTrend
indicator-parabolic-sar = Parabolic SAR
indicator-parabolic-sar-short = SAR
indicator-dmi = ADX / DMI (yön hareketi)
indicator-dmi-short = DMI
indicator-ichimoku = Ichimoku Bulutu
indicator-ichimoku-short = Ichimoku
indicator-rsi = Göreceli Güç Endeksi (RSI)
indicator-rsi-short = RSI
indicator-macd = MACD
indicator-macd-short = MACD
indicator-stochastic = Stokastik
indicator-stochastic-short = Stoch
indicator-stoch-rsi = Stokastik RSI
indicator-stoch-rsi-short = Stoch RSI
indicator-cci = Emtia Kanal Endeksi (CCI)
indicator-cci-short = CCI
indicator-atr = Ortalama Gerçek Aralık (ATR)
indicator-atr-short = ATR
indicator-volume = Hacim
indicator-volume-short = Hacim
indicator-buy-sell-volume = Alış/Satış Hacmi
indicator-buy-sell-volume-short = Alış/Satış
indicator-cvd = Kümülatif Hacim Deltası (CVD)
indicator-cvd-short = CVD
indicator-obv = Denge Hacmi (OBV)
indicator-obv-short = OBV
indicator-vwap = Hacim Ağırlıklı Ortalama Fiyat (VWAP)
indicator-vwap-short = VWAP

indicator-group-moving-averages = Hareketli ortalamalar
indicator-group-bands = Bantlar ve kanallar
indicator-group-trend = Trend
indicator-group-momentum = Momentum
indicator-group-volatility = Volatilite
indicator-group-volume = Hacim

indicator-param-length = Uzunluk
indicator-param-source = Kaynak
indicator-param-multiplier = Çarpan
indicator-param-atr-length = ATR uzunluğu
indicator-param-factor = Faktör
indicator-param-start = Başlangıç
indicator-param-increment = Artış
indicator-param-maximum = Üst sınır
indicator-param-di-length = DI uzunluğu
indicator-param-adx-smoothing = ADX yumuşatma
indicator-param-conversion = Dönüşüm çizgisi uzunluğu
indicator-param-base = Taban çizgisi uzunluğu
indicator-param-span-b = Öncü B uzunluğu
indicator-param-displacement = Kaydırma
indicator-param-fast = Hızlı uzunluk
indicator-param-slow = Yavaş uzunluk
indicator-param-signal = Sinyal uzunluğu
indicator-param-k-length = %K uzunluğu
indicator-param-k-smoothing = %K yumuşatma
indicator-param-d-smoothing = %D yumuşatma
indicator-param-rsi-length = RSI uzunluğu
indicator-param-stoch-length = Stokastik uzunluğu
indicator-param-anchor = Sıfırlanma

indicator-output-value = Değer
indicator-output-basis = Orta bant
indicator-output-upper = Üst bant
indicator-output-lower = Alt bant
indicator-output-up = Yükseliş
indicator-output-down = Düşüş
indicator-output-adx = ADX
indicator-output-plus-di = +DI
indicator-output-minus-di = −DI
indicator-output-conversion = Dönüşüm çizgisi
indicator-output-base = Taban çizgisi
indicator-output-lagging = Gecikmeli çizgi
indicator-output-lead-a = Öncü A
indicator-output-lead-b = Öncü B
indicator-output-histogram = Histogram
indicator-output-macd = MACD
indicator-output-signal = Sinyal
indicator-output-k = %K
indicator-output-d = %D
indicator-output-buy = Alış
indicator-output-sell = Satış

indicator-source-open = Açılış
indicator-source-high = En yüksek
indicator-source-low = En düşük
indicator-source-close = Kapanış
indicator-source-hl2 = hl2
indicator-source-hlc3 = hlc3
indicator-source-ohlc4 = ohlc4

indicator-choice-day = Gün
indicator-choice-week = Hafta
indicator-choice-month = Ay
indicator-choice-none = Hiç
