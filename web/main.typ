#{
  asset("styles.css", read("styles.css", encoding: none))
  asset("reset.css", read("reset.css", encoding: none))

  asset("android-chrome-192x192.png", read("favicons/android-chrome-192x192.png", encoding: none))
  asset("android-chrome-512x512.png", read("favicons/android-chrome-512x512.png", encoding: none))
  asset("apple-touch-icon.png", read("favicons/apple-touch-icon.png", encoding: none))
  asset("favicon-16x16.png", read("favicons/favicon-16x16.png", encoding: none))
  asset("favicon-32x32.png", read("favicons/favicon-32x32.png", encoding: none))
  asset("favicon.ico", read("favicons/favicon.ico", encoding: none))
  asset("site.webmanifest", read("favicons/site.webmanifest", encoding: none))

  include "index.typ"
  include "sikora-route.typ"
}
