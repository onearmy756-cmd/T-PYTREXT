# Mobile support for PyTreX

This branch adds scaffolding to build mobile apps from the existing frontend via three approaches:

1) PWA (progressive web app)
   - `frontend/manifest.webmanifest` and `frontend/service-worker.js` have been added.
   - `frontend/index.html` now registers the service worker and includes the manifest.
   - To test: serve the `frontend` folder on a static server. Example:
     - `npx http-server frontend -p 8000`
     - Open `http://<device-ip>:8000` from a mobile browser and install as PWA.

2) Capacitor (wrap web frontend as native app)
   - `capacitor.config.json` and a `package.json` with helper scripts were added at repo root.
   - Quick steps:
     - `npm install` (if you want local npm scripts)
     - `npm run cap:init` (only first time)
     - `npm run build:web` (copies/uses `frontend` as webDir)
     - `npm run cap:add-android`
     - `npm run cap:open:android` (open Android Studio and run)
   - Notes: iOS requires macOS and code signing setup.

3) React Native (Expo) wrapper
   - A minimal Expo app is in `mobile-react-native/`.
   - It uses a WebView to load the running frontend during development.
   - Quick steps:
     - `cd mobile-react-native`
     - `npm install`
     - `npx expo start` and use Expo Go on device or emulator.

Important notes and next steps
- Icons: existing `icons/icon.ico` is referenced. For best experience add PNG icons (192/512) in `frontend/icons/`.
- Service worker paths are relative to `frontend/`; if you serve from a different root, adjust URLs.
- For production native builds you should copy the built frontend into the native project's web folder (`www`) and configure native permissions (camera, storage) for features such as camera-based AI scanning.

If this looks good I can:
- Add PNG icons to `frontend/icons/` (please confirm sizes you want).
- Add a simple GitHub Actions workflow to build Android (.apk) on CI.
- Tweak the WebView RN app to bundle a local copy of the frontend for offline native builds.

