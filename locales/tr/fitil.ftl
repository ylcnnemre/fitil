# Fitil — Türkçe arayüz metinleri.
# Her anahtar en/fitil.ftl dosyasında da bulunmalıdır.

app-title = Fitil
app-version = Sürüm { $version }

language-tr = Türkçe
language-en = English

market-spot = Spot
market-futures = Vadeli (USDⓈ-M)

search-placeholder = Sembol ara…

watchlist-symbol = Sembol
watchlist-price = Fiyat
watchlist-change = 24s değişim
watchlist-loading = Semboller yükleniyor…
watchlist-empty = Eşleşen sembol yok.
watchlist-retry = Tekrar dene
watchlist-count = { $total } sembolden { $shown } tanesi, hacme göre sıralı

detail-empty = Grafiğini görmek için soldan bir sembol seç.
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
chart-hint = Tekerlek: yakınlaştır · Sürükle: kaydır · Fiyat eksenini sürükle: dikey ölçek · Zaman eksenini sürükle: yatay ölçek · Panel kenarını sürükle: panel yüksekliği · Çift tıkla: sıfırla
chart-indicators = İndikatörler
chart-volume-toggle = Hacim
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
