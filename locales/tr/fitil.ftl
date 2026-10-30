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

detail-empty = Ayrıntıları görmek için soldan bir sembol seç.
detail-timeframe = Zaman dilimi
detail-source = { $source } mumlardan üretiliyor
detail-open-candle = Açık mum
detail-ohlc = A { $open }   Y { $high }   D { $low }   K { $close }
detail-closes-in = Kapanışa kalan: { $time }
detail-waiting = Canlı veri bekleniyor…
detail-cached = Önbellekte { $count } mum ({ $timeframe })
detail-built = Bunlardan { $count } mum ({ $timeframe }) üretildi
detail-oldest = En eski: { $time }
detail-newest = En yeni: { $time }
detail-complete = Sembolün tüm geçmişi indirildi.
detail-last-load = Son yükleme: önbellekten { $cached }, Binance'ten { $downloaded } mum
detail-load-older = Daha eski mumları yükle
detail-loading = Yükleniyor…

status-connecting = Canlı veriye bağlanıyor…
status-connected = Canlı veri bağlı
status-reconnecting = Bağlantı koptu, { $seconds } sn sonra yeniden denenecek ({ $attempt }. deneme)
status-weight = İstek ağırlığı: { $used } / { $limit }
status-clock = Saat farkı: { $seconds } sn
status-cache = Önbellek: { $path }

error-restricted = Binance bulunduğun yerden erişime izin vermiyor.
error-rate-limited = Binance çok fazla istek gönderildiğini bildirdi. { $seconds } sn bekleyip tekrar dene.
error-network = Binance'e ulaşılamadı. İnternet bağlantını kontrol et.
error-exchange = Binance bir hata döndürdü: { $detail }
error-storage = Yerel önbellek kullanılamıyor: { $detail }
