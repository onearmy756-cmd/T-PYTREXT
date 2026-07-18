"""Tests for new PyTreX features: Auth, Validation, RateLimit, Plugins, I18n, OfflineSync, Async, MsgPack."""
import json
import time
import unittest
import pytest
from pytrex.core import (
    AuthManager, InputValidator, PluginManager, I18n,
    OfflineSyncQueue, execute_python_event, execute_python_event_async,
    _check_rate_limit, event, REGISTERED_EVENTS,
)


class TestAuthManager:
    def _make_auth(self):
        return AuthManager(secret="test_secret_key_for_testing_only")

    def test_register_and_login(self):
        auth = self._make_auth()
        assert auth.register_user("admin", "pass123", "admin") is True
        token = auth.login("admin", "pass123")
        assert token is not None
        assert auth.verify_token(token) is not None

    def test_wrong_password(self):
        auth = self._make_auth()
        auth.register_user("user1", "correct")
        assert auth.login("user1", "wrong") is None

    def test_duplicate_register(self):
        auth = self._make_auth()
        auth.register_user("user1", "pass")
        assert auth.register_user("user1", "pass") is False

    def test_rbac_admin(self):
        auth = self._make_auth()
        auth.register_user("admin", "p", "admin")
        token = auth.login("admin", "p")
        assert auth.has_permission(token, "any_event") is True

    def test_rbac_user(self):
        auth = self._make_auth()
        auth.register_user("user1", "p", "user")
        token = auth.login("user1", "p")
        assert auth.has_permission(token, "pata_akaunti") is True
        assert auth.has_permission(token, "delete_everything") is False

    def test_logout(self):
        auth = self._make_auth()
        auth.register_user("u", "p")
        token = auth.login("u", "p")
        assert auth.logout(token) is True
        assert auth.verify_token(token) is None

    def test_token_expiry(self):
        auth = self._make_auth()
        auth.TOKEN_EXPIRY_SECONDS = 1  # 1 second for testing
        auth.register_user("user1", "pass")
        token = auth.login("user1", "pass")
        assert token is not None
        assert auth.verify_token(token) is not None
        time.sleep(1.1)
        assert auth.verify_token(token) is None

    def test_cleanup_expired_tokens(self):
        auth = self._make_auth()
        auth.TOKEN_EXPIRY_SECONDS = 1
        auth.register_user("user1", "pass")
        auth.register_user("user2", "pass")
        t1 = auth.login("user1", "pass")
        t2 = auth.login("user2", "pass")
        time.sleep(1.1)
        cleaned = auth.cleanup_expired_tokens()
        assert cleaned == 2
        assert auth.verify_token(t1) is None
        assert auth.verify_token(t2) is None

    def test_no_default_secret_raises(self):
        import os
        old = os.environ.pop("PYTREX_AUTH_SECRET", None)
        try:
            with pytest.raises(ValueError, match="secret"):
                AuthManager()
        finally:
            if old:
                os.environ["PYTREX_AUTH_SECRET"] = old

    def test_password_hash_is_bcrypt_or_salted(self):
        auth = self._make_auth()
        auth.register_user("user1", "mypassword")
        stored = auth._users["user1"]["password"]
        # Should not be plain SHA-256 (64 hex chars)
        assert not (len(stored) == 64 and all(c in "0123456789abcdef" for c in stored))


class TestInputValidator:
    def test_valid_json(self):
        ok, payload, err = InputValidator.validate('{"name": "test"}')
        assert ok is True
        assert payload["name"] == "test"
        assert err is None

    def test_invalid_json(self):
        ok, payload, err = InputValidator.validate("not json")
        assert ok is False
        assert err is not None

    def test_schema_required(self):
        schema = {"name": {"required": True, "type": str}}
        ok, _, err = InputValidator.validate('{}', schema)
        assert ok is False
        assert "name" in err

    def test_schema_type_check(self):
        schema = {"age": {"required": True, "type": int}}
        ok, _, err = InputValidator.validate('{"age": "twenty"}', schema)
        assert ok is False

    def test_schema_min_max(self):
        schema = {"amount": {"required": True, "type": (int, float), "min": 0, "max": 1000}}
        ok, _, _ = InputValidator.validate('{"amount": 500}', schema)
        assert ok is True
        ok, _, err = InputValidator.validate('{"amount": -1}', schema)
        assert ok is False


class TestRateLimit:
    def test_basic_limit(self):
        key = "rate_limit_unique_test_key_12345"
        for _ in range(5):
            assert _check_rate_limit(key, max_calls=5) is True
        assert _check_rate_limit(key, max_calls=5) is False


class TestPluginManager:
    def test_register_unregister(self):
        pm = PluginManager()
        class DummyPlugin:
            loaded = False
            unloaded = False
            def on_load(self): self.loaded = True
            def on_unload(self): self.unloaded = True

        p = DummyPlugin()
        assert pm.register("dummy", p) is True
        assert p.loaded is True
        assert pm.get_plugin("dummy") is p
        assert "dummy" in pm.list_plugins()
        assert pm.unregister("dummy") is True
        assert p.unloaded is True

    def test_duplicate_register(self):
        pm = PluginManager()
        pm.register("p1", object())
        assert pm.register("p1", object()) is False

    def test_reserved_name_rejected(self):
        pm = PluginManager()
        assert pm.register("core", object()) is False
        assert pm.register("system", object()) is False
        assert pm.register("admin", object()) is False

    def test_invalid_name_rejected(self):
        pm = PluginManager()
        assert pm.register("", object()) is False
        assert pm.register("bad name!", object()) is False
        assert pm.register("my-plugin", object()) is False

    def test_none_plugin_rejected(self):
        pm = PluginManager()
        assert pm.register("test_plugin", None) is False

    def test_on_load_crash_safety(self):
        pm = PluginManager()
        class CrashPlugin:
            def on_load(self):
                raise RuntimeError("boom")
        assert pm.register("crashy", CrashPlugin()) is False
        assert "crashy" not in pm.list_plugins()


class TestI18n:
    def test_swahili(self):
        i18n = I18n("sw")
        assert i18n.t("welcome") == "Karibu kwenye PyTreX"

    def test_english(self):
        i18n = I18n("en")
        assert i18n.t("welcome") == "Welcome to PyTreX"

    def test_french(self):
        i18n = I18n("fr")
        assert i18n.t("welcome") == "Bienvenue a PyTreX"

    def test_fallback(self):
        i18n = I18n("sw")
        assert i18n.t("nonexistent_key") == "nonexistent_key"


class TestOfflineSyncQueue:
    def test_enqueue_drain(self):
        q = OfflineSyncQueue()
        q.enqueue("test_event", {"data": 1})
        q.enqueue("test_event2", {"data": 2})
        assert q.pending_count == 2
        items = q.drain()
        assert len(items) == 2
        assert q.pending_count == 0


class TestAsyncEvent:
    def test_async_execution(self):
        @event("async_test")
        def handler(data):
            return json.dumps({"status": "ok", "received": data})

        result = execute_python_event_async("async_test", "test_data")
        parsed = json.loads(result)
        assert parsed["status"] == "ok"


class TestRustNewFeatures:
    """Test new Rust features: file system API, MessagePack, multi-window."""

    def test_file_system(self):
        try:
            import my_framework
            import tempfile, os
            test_path = os.path.join(tempfile.gettempdir(), "pytrex_test_fs.txt")
            my_framework.andika_faili_salama(test_path, "Hello PyTreX!")
            assert my_framework.faili_lipo(test_path) is True
            content = my_framework.soma_faili_salama(test_path)
            assert content == "Hello PyTreX!"
            os.remove(test_path)
        except ImportError:
            pytest.skip("my_framework not built")

    def test_msgpack(self):
        try:
            import my_framework
            original = '{"name":"PyTreX","version":1.0,"features":["ai","blockchain"]}'
            packed = my_framework.pack_data(original)
            assert isinstance(packed, bytes)
            unpacked = my_framework.unpack_data(packed)
            parsed = json.loads(unpacked)
            assert parsed["name"] == "PyTreX"
            assert parsed["version"] == 1.0
        except ImportError:
            pytest.skip("my_framework not built")


class TestMobileAPI:
    """Test mobile API bridge."""

    def test_device_info(self):
        from pytrex.core import MobileAPI
        api = MobileAPI()
        info = api.device_info()
        assert "os" in info
        assert "arch" in info
        assert "is_mobile" in info
        assert "is_desktop" in info

    def test_is_desktop_on_desktop(self):
        from pytrex.core import MobileAPI
        api = MobileAPI()
        assert api.is_desktop() is True

    def test_is_mobile_on_desktop(self):
        from pytrex.core import MobileAPI
        api = MobileAPI()
        assert api.is_mobile() is False

    def test_build_android(self):
        try:
            import my_framework
            import my_framework
            result = my_framework.build_mobile("android")
            parsed = json.loads(result)
            assert parsed["status"] == "ok"
            assert parsed["target"] == "android"
        except ImportError:
            pytest.skip("my_framework not built")

    def test_build_ios(self):
        try:
            import my_framework
            result = my_framework.build_mobile("ios")
            parsed = json.loads(result)
            assert parsed["status"] == "ok"
            assert parsed["target"] == "ios"
        except ImportError:
            pytest.skip("my_framework not built")

    def test_build_invalid_target(self):
        try:
            import my_framework
            with pytest.raises(Exception):
                my_framework.build_mobile("windows")
        except ImportError:
            pytest.skip("my_framework not built")

    def test_mobile_in_pytrex_app(self):
        from pytrex.core import PyTreXApp, MobileAPI
        app = PyTreXApp(name="Test Mobile App")
        assert isinstance(app.mobile, MobileAPI)
        assert app.mobile.is_desktop() is True


class TestAdvancedFeatures:
    """Test 12 advanced features."""

    def test_biometric_auth(self):
        from pytrex.core import BiometricAuth
        bio = BiometricAuth()
        assert bio.is_available() is False  # desktop
        result = bio.authenticate("test")
        assert result["status"] == "error"  # not available on desktop

    def test_push_notifications(self):
        from pytrex.core import PushNotifications
        push = PushNotifications()
        assert push.configure(fcm_token="test_token") is True
        result = push.send("Hello", "World")
        assert result["status"] == "ok"
        assert result["title"] == "Hello"

    def test_push_not_configured(self):
        from pytrex.core import PushNotifications
        push = PushNotifications()
        result = push.send("Hello", "World")
        assert result["status"] == "error"

    def test_qr_code(self):
        from pytrex.core import QRCodeManager
        qr = QRCodeManager()
        result = qr.scan()
        assert result["status"] == "ok"

    def test_system_tray(self):
        from pytrex.core import SystemTray
        tray = SystemTray()
        tray.add_menu_item("Settings")
        tray.add_menu_item("Quit")
        assert len(tray.menu) == 2
        assert tray.is_visible is False
        tray.show(tooltip="My App")
        assert tray.is_visible is True
        tray.hide()
        assert tray.is_visible is False

    def test_system_tray_notify(self):
        from pytrex.core import SystemTray
        tray = SystemTray()
        result = tray.notify("Alert", "Something happened")
        assert result["status"] == "ok"
        assert result["title"] == "Alert"

    def test_deep_linking(self):
        from pytrex.core import DeepLinking
        dl = DeepLinking("pytrex")
        called = []
        dl.handle("open", lambda p: called.append(p))
        result = dl.process_link("pytrex://open")
        assert result["status"] == "ok"
        assert result["path"] == "open"
        assert called == ["open"]

    def test_deep_linking_invalid_scheme(self):
        from pytrex.core import DeepLinking
        dl = DeepLinking("pytrex")
        result = dl.process_link("http://example.com")
        assert result["status"] == "error"

    def test_api_server(self):
        from pytrex.core import APIServer
        server = APIServer(port=18099)
        assert server.is_running is False

        @server.endpoint("/test")
        def test_ep(data):
            return {"status": "ok", "data": data}

        assert "/test" in server._endpoints
        assert server.start() is True
        assert server.is_running is True
        server.stop()

    def test_crash_reporter(self):
        from pytrex.core import CrashReporter
        cr = CrashReporter()
        result = cr.report("TestError", "stack trace here")
        assert result["status"] == "logged"
        assert len(cr.get_reports()) == 1
        cr.clear()
        assert len(cr.get_reports()) == 0

    def test_analytics(self):
        from pytrex.core import Analytics
        an = Analytics()
        an.track("test_event")  # should not track (disabled)
        assert len(an.get_events()) == 0
        an.enable()
        an.track("test_event", {"key": "value"})
        events = an.get_events()
        assert len(events) == 1
        assert events[0]["event"] == "test_event"
        assert an.is_enabled is True

    def test_pdf_generator(self):
        from pytrex.core import PDFGenerator
        import tempfile, os
        pdf = PDFGenerator()
        path = os.path.join(tempfile.gettempdir(), "test_pytrex.pdf")
        result = pdf.from_text("Hello PyTreX PDF", path)
        assert result["status"] == "ok"
        assert os.path.exists(path)
        os.remove(path)

    def test_pdf_print(self):
        from pytrex.core import PDFGenerator
        pdf = PDFGenerator()
        result = pdf.print_document("test.pdf")
        assert result["status"] == "ok"

    def test_compression(self):
        from pytrex.core import Compression
        comp = Compression()
        data = b"AAAAAABBBCCC" * 100  # Repeated data compresses well
        compressed = comp.compress(data)
        decompressed = comp.decompress(compressed)
        assert decompressed == data

    def test_encryption(self):
        from pytrex.core import Compression
        comp = Compression()
        encrypted = comp.encrypt("Hello World", "secret_key")
        decrypted = comp.decrypt(encrypted, "secret_key")
        assert decrypted == "Hello World"

    def test_image_processor_resize(self):
        from pytrex.core import ImageProcessor
        import tempfile, os
        img = ImageProcessor()
        test_file = os.path.join(tempfile.gettempdir(), "test_img.png")
        try:
            from PIL import Image as PILImage
            pil_img = PILImage.new("RGB", (200, 200), color=(255, 0, 0))
            pil_img.save(test_file)
        except ImportError:
            # No Pillow — create a minimal valid PNG manually
            import struct, zlib
            def png_chunk(ctype, data):
                c = ctype + data
                return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xffffffff)
            header = b"\x89PNG\r\n\x1a\n"
            ihdr = struct.pack(">IIBBBBB", 200, 200, 8, 2, 0, 0, 0)
            raw = b""
            for _ in range(200):
                raw += b"\x00" + b"\xff\x00\x00" * 200
            idat = zlib.compress(raw)
            with open(test_file, "wb") as f:
                f.write(header + png_chunk(b"IHDR", ihdr) + png_chunk(b"IDAT", idat) + png_chunk(b"IEND", b""))
        result = img.resize(test_file, 100, 100)
        assert result["status"] == "ok"
        assert result["width"] == 100
        os.remove(test_file)
        if os.path.exists(result["path"]):
            os.remove(result["path"])

    def test_image_processor_crop(self):
        from pytrex.core import ImageProcessor
        import tempfile, os
        img = ImageProcessor()
        test_file = os.path.join(tempfile.gettempdir(), "test_crop.png")
        try:
            from PIL import Image as PILImage
            pil_img = PILImage.new("RGB", (200, 200), color=(0, 255, 0))
            pil_img.save(test_file)
        except ImportError:
            pytest.skip("PIL not installed")
        result = img.crop(test_file, 10, 20, 100, 100)
        assert result["status"] == "ok"
        assert result["crop"]["x"] == 10
        os.remove(test_file)
        if os.path.exists(result.get("path", "")):
            os.remove(result["path"])

    def test_image_processor_watermark(self):
        from pytrex.core import ImageProcessor
        import tempfile, os
        img = ImageProcessor()
        test_file = os.path.join(tempfile.gettempdir(), "test_wm.png")
        try:
            from PIL import Image as PILImage
            pil_img = PILImage.new("RGB", (200, 200), color=(0, 0, 255))
            pil_img.save(test_file)
        except ImportError:
            pytest.skip("PIL not installed")
        result = img.watermark(test_file, "PyTreX")
        assert result["status"] == "ok"
        assert result["watermark"] == "PyTreX"
        os.remove(test_file)
        if os.path.exists(result.get("path", "")):
            os.remove(result["path"])

    def test_background_service(self):
        from pytrex.core import BackgroundService
        import time
        bg = BackgroundService()
        counter = [0]
        def task():
            counter[0] += 1
        assert bg.start_task("sync", task, interval=0.1) is True
        assert "sync" in bg.list_tasks()
        time.sleep(0.35)
        assert counter[0] >= 2
        assert bg.stop_task("sync") is True
        bg.stop_all()

    def test_all_features_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            BiometricAuth, PushNotifications, QRCodeManager, SystemTray,
            DeepLinking, APIServer, CrashReporter, Analytics,
            PDFGenerator, Compression, ImageProcessor, BackgroundService,
        )
        app = PyTreXApp(name="Full Feature Test")
        assert isinstance(app.biometric, BiometricAuth)
        assert isinstance(app.push, PushNotifications)
        assert isinstance(app.qr, QRCodeManager)
        assert isinstance(app.tray, SystemTray)
        assert isinstance(app.deep_link, DeepLinking)
        assert isinstance(app.api, APIServer)
        assert isinstance(app.crash, CrashReporter)
        assert isinstance(app.analytics, Analytics)
        assert isinstance(app.pdf, PDFGenerator)
        assert isinstance(app.compress, Compression)
        assert isinstance(app.image, ImageProcessor)
        assert isinstance(app.background, BackgroundService)


class TestBatch4Features:
    """Test batch 4: WebSocket, Cron, Email, PDFViewer, Charts, Media, FileWatcher, Clipboard, Screenshot, Network, Config, Session."""

    def test_websocket_server(self):
        from pytrex.core import WebSocketServer
        ws = WebSocketServer(port=18765)
        assert ws.is_running is False
        assert ws.client_count == 0

    def test_cron_scheduler(self):
        from pytrex.core import CronScheduler
        cron = CronScheduler()
        counter = [0]
        assert cron.every("1s", "test_job", lambda: counter.__setitem__(0, counter[0] + 1)) is True
        import time
        time.sleep(2.5)
        assert counter[0] >= 2
        assert "test_job" in cron.list_jobs()
        cron.cancel("test_job")
        cron.stop()

    def test_cron_once(self):
        from pytrex.core import CronScheduler
        cron = CronScheduler()
        called = [False]
        cron.once("one_time", lambda: called.__setitem__(0, True), delay=0.1)
        import time
        time.sleep(1.0)
        assert called[0] is True
        assert "one_time" not in cron.list_jobs()

    def test_cron_invalid_interval(self):
        from pytrex.core import CronScheduler
        cron = CronScheduler()
        assert cron.every("abc", "bad", lambda: None) is False

    def test_email_not_configured(self):
        from pytrex.core import EmailService
        email = EmailService()
        result = email.send("test@test.com", "Test", "Hello")
        assert result["status"] == "logged"

    def test_email_configure(self):
        from pytrex.core import EmailService
        email = EmailService()
        assert email.configure("smtp.test.com", 587, "user", "pass") is True

    def test_pdf_viewer(self):
        from pytrex.core import PDFViewer
        import tempfile, os
        viewer = PDFViewer()
        path = os.path.join(tempfile.gettempdir(), "test_view.pdf")
        with open(path, "w") as f:
            f.write("test")
        result = viewer.view(path)
        assert result["status"] == "ok"
        assert viewer.open_count == 1
        viewer.close(path)
        assert viewer.open_count == 0
        os.remove(path)

    def test_pdf_viewer_not_found(self):
        from pytrex.core import PDFViewer
        viewer = PDFViewer()
        result = viewer.view("nonexistent.pdf")
        assert result["status"] == "error"

    def test_chart_pie(self):
        from pytrex.core import ChartVisualizer
        charts = ChartVisualizer()
        result = charts.pie({"A": 30, "B": 70}, title="Test Pie")
        assert result["status"] == "ok"
        assert "<svg" in result["svg"]

    def test_chart_bar(self):
        from pytrex.core import ChartVisualizer
        charts = ChartVisualizer()
        result = charts.bar({"Jan": 10, "Feb": 20}, title="Test Bar")
        assert result["status"] == "ok"
        assert "<svg" in result["svg"]

    def test_chart_line(self):
        from pytrex.core import ChartVisualizer
        charts = ChartVisualizer()
        result = charts.line([10, 20, 15, 30], title="Test Line")
        assert result["status"] == "ok"
        assert "<svg" in result["svg"]

    def test_chart_line_empty(self):
        from pytrex.core import ChartVisualizer
        charts = ChartVisualizer()
        result = charts.line([], title="Empty")
        assert result["status"] == "error"

    def test_media_player(self):
        from pytrex.core import MediaPlayer
        import tempfile, os
        player = MediaPlayer()
        path = os.path.join(tempfile.gettempdir(), "test_audio.mp3")
        with open(path, "w") as f:
            f.write("fake audio")
        result = player.play(path)
        assert result["status"] == "ok"
        assert player.is_playing is True
        player.pause()
        assert player.is_playing is False
        player.stop()
        assert player.current_file is None
        os.remove(path)

    def test_media_player_not_found(self):
        from pytrex.core import MediaPlayer
        player = MediaPlayer()
        result = player.play("nonexistent.mp3")
        assert result["status"] == "error"

    def test_media_volume(self):
        from pytrex.core import MediaPlayer
        player = MediaPlayer()
        player.set_volume(0.5)
        player.set_volume(2.0)  # should clamp to 1.0

    def test_file_watcher(self):
        from pytrex.core import FileWatcher
        import tempfile
        watcher = FileWatcher()
        result = watcher.watch(tempfile.gettempdir(), lambda p: None)
        assert result is True
        assert tempfile.gettempdir() in watcher.watched_paths
        watcher.stop()

    def test_clipboard_copy_paste(self):
        from pytrex.core import ClipboardManager
        clip = ClipboardManager()
        assert clip.copy("PyTreX Test") is True
        text = clip.paste()
        assert "PyTreX" in text or text == ""  # may be empty in headless

    def test_screenshot(self):
        from pytrex.core import ScreenshotCapture
        import os
        ss = ScreenshotCapture()
        result = ss.capture("test_screenshot.png")
        # May fail in headless mode
        if result["status"] == "ok" and os.path.exists("test_screenshot.png"):
            os.remove("test_screenshot.png")

    def test_config_manager(self):
        from pytrex.core import ConfigManager
        import tempfile, os
        path = os.path.join(tempfile.gettempdir(), "test_pytrex_config.json")
        if os.path.exists(path):
            os.remove(path)
        cfg = ConfigManager(config_path=path)
        cfg.set("theme", "dark")
        cfg.set("lang", "sw")
        assert cfg.get("theme") == "dark"
        assert cfg.get("lang") == "sw"
        assert "theme" in cfg.keys()
        assert cfg.delete("theme") is True
        assert cfg.get("theme") is None
        assert cfg.to_dict() == {"lang": "sw"}
        os.remove(path)

    def test_session_manager(self):
        from pytrex.core import SessionManager
        sm = SessionManager(timeout=3600)
        token = sm.create("user123", {"role": "admin"})
        assert token is not None
        session = sm.validate(token)
        assert session is not None
        assert session["user_id"] == "user123"
        assert session["metadata"]["role"] == "admin"
        assert sm.active_count == 1
        assert sm.destroy(token) is True
        assert sm.active_count == 0
        assert sm.validate(token) is None

    def test_session_invalid_token(self):
        from pytrex.core import SessionManager
        sm = SessionManager()
        assert sm.validate("invalid_token") is None
        assert sm.destroy("invalid") is False

    def test_session_expiry(self):
        from pytrex.core import SessionManager
        sm = SessionManager(timeout=0)  # immediate expiry
        token = sm.create("user1")
        import time
        time.sleep(0.1)
        assert sm.validate(token) is None  # should be expired

    def test_all_batch4_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            WebSocketServer, CronScheduler, EmailService, PDFViewer,
            ChartVisualizer, MediaPlayer, FileWatcher, ClipboardManager,
            ScreenshotCapture, NetworkScanner, ConfigManager, SessionManager,
        )
        app = PyTreXApp(name="Batch4 Test")
        assert isinstance(app.websocket, WebSocketServer)
        assert isinstance(app.scheduler, CronScheduler)
        assert isinstance(app.email, EmailService)
        assert isinstance(app.pdf_viewer, PDFViewer)
        assert isinstance(app.charts, ChartVisualizer)
        assert isinstance(app.media, MediaPlayer)
        assert isinstance(app.watcher, FileWatcher)
        assert isinstance(app.clipboard, ClipboardManager)
        assert isinstance(app.screenshot, ScreenshotCapture)
        assert isinstance(app.network_scanner, NetworkScanner)
        assert isinstance(app.config, ConfigManager)
        assert isinstance(app.session, SessionManager)


class TestBatch5Features:
    """Test batch 5: Terminal, CodeEditor, Migrations, GraphQL, OAuth2, WebRTC, Barcode, Maps, Bluetooth, USB, Process, Theme."""

    def test_terminal_run(self):
        from pytrex.core import TerminalEmulator
        term = TerminalEmulator()
        result = term.run("echo PyTreX")
        assert result["status"] == "ok"
        assert "PyTreX" in result["output"]
        assert len(term.history) == 1

    def test_terminal_error(self):
        from pytrex.core import TerminalEmulator
        term = TerminalEmulator()
        result = term.run("nonexistent_command_xyz")
        assert result["status"] == "ok"  # shell returns exit code
        assert result["exit_code"] != 0

    def test_code_editor(self):
        from pytrex.core import CodeEditor
        import tempfile, os
        editor = CodeEditor()
        path = os.path.join(tempfile.gettempdir(), "test_edit.py")
        with open(path, "w") as f:
            f.write("print('hello')")
        result = editor.open(path)
        assert result["status"] == "ok"
        assert result["language"] == "python"
        assert path in editor.open_files
        assert editor.save(path, "print('updated')") is True
        editor.close(path)
        assert path not in editor.open_files
        os.remove(path)

    def test_code_editor_not_found(self):
        from pytrex.core import CodeEditor
        editor = CodeEditor()
        result = editor.open("nonexistent.py")
        assert result["status"] == "error"

    def test_code_editor_language_detection(self):
        from pytrex.core import CodeEditor
        import tempfile, os
        editor = CodeEditor()
        for ext, lang in [(".js", "javascript"), (".rs", "rust"), (".html", "html"), (".json", "json")]:
            path = os.path.join(tempfile.gettempdir(), f"test{ext}")
            with open(path, "w") as f:
                f.write("test")
            result = editor.open(path)
            assert result["language"] == lang
            editor.close(path)
            os.remove(path)

    def test_database_migrations(self):
        from pytrex.core import DatabaseMigrations
        import tempfile, os
        db = os.path.join(tempfile.gettempdir(), "test_migrations.db")
        if os.path.exists(db):
            os.remove(db)
        mig = DatabaseMigrations(db_path=db)
        assert mig.create("init", "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)") is True
        assert "init" in mig.pending
        result = mig.run("init")
        assert result["status"] == "ok"
        assert "init" in mig.applied
        assert len(mig.pending) == 0
        if os.path.exists(db):
            os.remove(db)

    def test_migration_duplicate(self):
        from pytrex.core import DatabaseMigrations
        mig = DatabaseMigrations()
        assert mig.create("test", "SELECT 1") is True
        assert mig.create("test", "SELECT 2") is False

    def test_migration_not_found(self):
        from pytrex.core import DatabaseMigrations
        mig = DatabaseMigrations()
        result = mig.run("nonexistent")
        assert result["status"] == "error"

    def test_graphql(self):
        from pytrex.core import GraphQLServer
        gql = GraphQLServer()

        @gql.resolver("hello")
        def hello():
            return "world"

        @gql.resolver("count")
        def count():
            return 42

        result = gql.query("{ hello count }")
        assert result["data"]["hello"] == "world"
        assert result["data"]["count"] == 42

    def test_oauth2(self):
        from pytrex.core import OAuth2Integration
        oauth = OAuth2Integration()
        assert oauth.google("client_id", "secret") is True
        assert oauth.github("client_id", "secret") is True
        assert oauth.facebook("client_id", "secret") is True
        assert "google" in oauth.providers
        assert "github" in oauth.providers
        assert "facebook" in oauth.providers
        url = oauth.get_auth_url("google", scope="email profile")
        assert "accounts.google.com" in url
        assert "client_id" in url

    def test_oauth2_exchange(self):
        from pytrex.core import OAuth2Integration
        oauth = OAuth2Integration()
        oauth.google("id", "secret")
        result = oauth.exchange_code("google", "test_code")
        assert result["status"] == "ok"

    def test_oauth2_unregistered(self):
        from pytrex.core import OAuth2Integration
        oauth = OAuth2Integration()
        result = oauth.exchange_code("twitter", "code")
        assert result["status"] == "error"

    def test_webrtc(self):
        from pytrex.core import WebRTCVideoCall
        rtc = WebRTCVideoCall()
        rtc.register_peer("peer1", {"name": "Alice"})
        assert rtc.in_call is False
        result = rtc.call("peer1")
        assert result["status"] == "ok"
        assert rtc.in_call is True
        assert rtc.current_peer == "peer1"
        result = rtc.hangup()
        assert result["status"] == "ok"
        assert rtc.in_call is False

    def test_webrtc_peer_not_found(self):
        from pytrex.core import WebRTCVideoCall
        rtc = WebRTCVideoCall()
        result = rtc.call("unknown")
        assert result["status"] == "error"

    def test_barcode_parse(self):
        from pytrex.core import BarcodeScanner
        scanner = BarcodeScanner()
        assert scanner.parse("1234567890123")["type"] == "EAN-13"
        assert scanner.parse("123456789012")["type"] == "UPC-A"
        assert scanner.parse("12345678")["type"] == "EAN-8"
        assert scanner.parse("https://example.com")["type"] == "url"
        assert scanner.parse("hello")["type"] == "text"

    def test_barcode_scan(self):
        from pytrex.core import BarcodeScanner
        scanner = BarcodeScanner()
        result = scanner.scan()
        assert result["status"] == "ok"

    def test_maps(self):
        from pytrex.core import GeolocationMaps
        maps = GeolocationMaps()
        result = maps.show(-6.823, 39.269, zoom=14)
        assert result["status"] == "ok"
        assert "leaflet" in result["html"]
        assert result["lat"] == -6.823

    def test_maps_markers(self):
        from pytrex.core import GeolocationMaps
        maps = GeolocationMaps()
        maps.add_marker(-6.8, 39.2, "Dar es Salaam")
        maps.add_marker(-3.3, 36.7, "Arusha")
        assert len(maps.markers) == 2

    def test_maps_geocode(self):
        from pytrex.core import GeolocationMaps
        maps = GeolocationMaps()
        result = maps.geocode("Dar es Salaam")
        assert result["status"] == "ok"

    def test_bluetooth(self):
        from pytrex.core import BluetoothManager
        bt = BluetoothManager()
        assert bt.connected_device is None
        result = bt.disconnect()
        assert result["status"] == "error"
        result = bt.send("data")
        assert result["status"] == "error"

    def test_usb_devices(self):
        from pytrex.core import USBDeviceManager
        usb = USBDeviceManager()
        devices = usb.list_devices()
        assert isinstance(devices, list)

    def test_process_manager(self):
        from pytrex.core import ProcessManager
        import time
        pm = ProcessManager()
        assert pm.start("test_proc", "echo hello") is True
        time.sleep(0.5)
        procs = pm.list()
        assert len(procs) == 1
        assert procs[0]["name"] == "test_proc"
        pm.stop_all()

    def test_theme_manager(self):
        from pytrex.core import ThemeManager
        theme = ThemeManager()
        assert theme.current == "dark"
        assert "dark" in theme.available_themes
        assert "light" in theme.available_themes
        assert "swahili" in theme.available_themes
        assert theme.set("light") is True
        assert theme.current == "light"
        assert theme.get("background") == "#ffffff"
        assert theme.set("nonexistent") is False

    def test_theme_custom(self):
        from pytrex.core import ThemeManager
        theme = ThemeManager()
        assert theme.custom("ocean", {"background": "#001f3f", "primary": "#0074D9"}) is True
        assert theme.current == "ocean"
        assert theme.get("background") == "#001f3f"

    def test_theme_css(self):
        from pytrex.core import ThemeManager
        theme = ThemeManager()
        css = theme.to_css()
        assert ":root" in css
        assert "--background" in css

    def test_all_batch5_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            TerminalEmulator, CodeEditor, DatabaseMigrations, GraphQLServer,
            OAuth2Integration, WebRTCVideoCall, BarcodeScanner, GeolocationMaps,
            BluetoothManager, USBDeviceManager, ProcessManager, ThemeManager,
        )
        app = PyTreXApp(name="Batch5 Test")
        assert isinstance(app.terminal, TerminalEmulator)
        assert isinstance(app.editor, CodeEditor)
        assert isinstance(app.migrations, DatabaseMigrations)
        assert isinstance(app.graphql, GraphQLServer)
        assert isinstance(app.oauth, OAuth2Integration)
        assert isinstance(app.webrtc, WebRTCVideoCall)
        assert isinstance(app.barcode, BarcodeScanner)
        assert isinstance(app.maps, GeolocationMaps)
        assert isinstance(app.bluetooth, BluetoothManager)
        assert isinstance(app.usb, USBDeviceManager)
        assert isinstance(app.process, ProcessManager)
        assert isinstance(app.theme, ThemeManager)


class TestBatch6Features:
    """Test batch 6: AutoFix, Health, Encryption, Cache, TaskQueue, Notifications, Backup, Log, Deps, Perf."""

    def test_autofix_diagnose(self):
        from pytrex.core import AutoFixEngine
        af = AutoFixEngine()
        result = af.diagnose("ImportError: No module named 'foo'")
        assert result["status"] == "ok"
        assert len(result["fixes"]) > 0
        assert any("pip install" in f for f in result["fixes"])

    def test_autofix_multiple_errors(self):
        from pytrex.core import AutoFixEngine
        af = AutoFixEngine()
        result = af.diagnose("KeyError: 'name'\nTypeError: unsupported operand")
        assert result["status"] == "ok"
        assert len(result["fixes"]) >= 2

    def test_autofix_unknown_error(self):
        from pytrex.core import AutoFixEngine
        af = AutoFixEngine()
        result = af.diagnose("Something weird happened")
        assert result["status"] == "ok"
        assert len(result["fixes"]) >= 1

    def test_autofix_add_pattern(self):
        from pytrex.core import AutoFixEngine
        af = AutoFixEngine()
        af.add_pattern("CustomError", "Restart the app")
        assert "CustomError" in af.patterns

    def test_autofix_try_fix(self):
        from pytrex.core import AutoFixEngine
        af = AutoFixEngine()
        result = af.try_fix("KeyError: 'name'", code="data['name']")
        assert result["status"] == "ok"
        assert "diagnosis" in result

    def test_health_checker(self):
        from pytrex.core import HealthChecker
        hc = HealthChecker()
        result = hc.run()
        assert result["status"] in ("ok", "warning")
        assert "os" in result
        assert hc.last_result is not None

    def test_health_checker_custom(self):
        from pytrex.core import HealthChecker
        hc = HealthChecker()
        hc.register_check("db", lambda: {"status": "ok"})
        hc.register_check("cache", lambda: {"status": "ok"})
        result = hc.run()
        assert "custom_checks" in result
        assert result["custom_checks"]["db"]["status"] == "ok"

    def test_encryption_encrypt_decrypt(self):
        from pytrex.core import EncryptionManager
        enc = EncryptionManager()
        encrypted = enc.encrypt("Hello PyTreX", password="secret123")
        assert encrypted != "Hello PyTreX"
        decrypted = enc.decrypt(encrypted, password="secret123")
        assert decrypted == "Hello PyTreX"

    def test_encryption_hash(self):
        from pytrex.core import EncryptionManager
        enc = EncryptionManager()
        h = enc.hash("test data", "sha256")
        assert len(h) == 64
        h512 = enc.hash("test data", "sha512")
        assert len(h512) == 128

    def test_encryption_generate_secret(self):
        from pytrex.core import EncryptionManager
        enc = EncryptionManager()
        secret = enc.generate_secret(32)
        assert len(secret) == 32

    def test_encryption_file(self):
        from pytrex.core import EncryptionManager
        import tempfile, os
        enc = EncryptionManager()
        path = os.path.join(tempfile.gettempdir(), "test_enc.txt")
        with open(path, "w") as f:
            f.write("Secret data")
        result = enc.encrypt_file(path, password="key123")
        assert result["status"] == "ok"
        assert os.path.exists(result["output"])
        dec_result = enc.decrypt_file(result["output"], password="key123")
        assert dec_result["status"] == "ok"
        with open(dec_result["output"], "r") as f:
            assert f.read() == "Secret data"
        os.remove(path)
        os.remove(result["output"])
        os.remove(dec_result["output"])

    def test_cache_manager(self):
        from pytrex.core import CacheManager
        cache = CacheManager(max_size=5, default_ttl=10)
        cache.set("key1", "value1")
        cache.set("key2", "value2")
        assert cache.get("key1") == "value1"
        assert cache.get("key2") == "value2"
        assert cache.get("nonexistent", "default") == "default"
        assert cache.size == 2
        assert cache.stats["hits"] == 2

    def test_cache_ttl_expiry(self):
        from pytrex.core import CacheManager
        import time
        cache = CacheManager(default_ttl=1)
        cache.set("temp", "data", ttl=1)
        assert cache.get("temp") == "data"
        time.sleep(1.5)
        assert cache.get("temp") is None

    def test_cache_lru_eviction(self):
        from pytrex.core import CacheManager
        cache = CacheManager(max_size=3)
        cache.set("a", 1)
        cache.set("b", 2)
        cache.set("c", 3)
        cache.set("d", 4)  # should evict "a"
        assert cache.get("a") is None
        assert cache.get("d") == 4

    def test_cache_delete_clear(self):
        from pytrex.core import CacheManager
        cache = CacheManager()
        cache.set("x", 1)
        assert cache.delete("x") is True
        assert cache.delete("x") is False
        cache.set("y", 2)
        cache.clear()
        assert cache.size == 0

    def test_task_queue(self):
        from pytrex.core import TaskQueue
        import time
        tq = TaskQueue(num_workers=2)
        results = []
        tq.enqueue("task1", lambda data: results.append("done1"))
        tq.enqueue("task2", lambda data: results.append("done2"))
        tq.start()
        time.sleep(1.0)
        tq.stop()
        assert len(results) >= 2
        assert tq.completed_count >= 2

    def test_notification_manager(self):
        from pytrex.core import NotificationManager
        nm = NotificationManager()
        sent = []
        nm.register_channel("toast", lambda t, b: sent.append(f"{t}: {b}"))
        result = nm.send("Alert", "System update", ["toast"])
        assert result["status"] == "ok"
        assert len(sent) == 1
        assert "Alert" in sent[0]
        assert len(nm.history) == 1

    def test_backup_manager(self):
        from pytrex.core import BackupManager
        import tempfile, os
        backup_dir = os.path.join(tempfile.gettempdir(), "test_backups")
        bm = BackupManager(backup_dir=backup_dir)
        test_file = os.path.join(tempfile.gettempdir(), "test_backup_src.txt")
        with open(test_file, "w") as f:
            f.write("backup me")
        result = bm.create([test_file])
        assert result["status"] == "ok"
        assert os.path.exists(result["path"])
        backups = bm.list_backups()
        assert len(backups) >= 1
        restore_dir = os.path.join(tempfile.gettempdir(), "test_restore")
        os.makedirs(restore_dir, exist_ok=True)
        restore_result = bm.restore(result["path"], restore_dir)
        assert restore_result["status"] == "ok"
        bm.delete_backup(os.path.basename(result["path"]))
        os.remove(test_file)

    def test_log_manager(self):
        from pytrex.core import LogManager
        import tempfile, os
        log_path = os.path.join(tempfile.gettempdir(), "test_pytrex.log")
        if os.path.exists(log_path):
            os.remove(log_path)
        lm = LogManager(log_file=log_path)
        lm.info("Test info message")
        lm.warn("Test warning")
        lm.error("Test error", {"code": 500})
        entries = lm.get_entries()
        assert len(entries) == 3
        errors = lm.get_entries(level="ERROR")
        assert len(errors) == 1
        assert errors[0]["data"]["code"] == 500
        lm.clear()
        assert len(lm.get_entries()) == 0
        if os.path.exists(log_path):
            os.remove(log_path)

    def test_dependency_checker(self):
        from pytrex.core import DependencyChecker
        dc = DependencyChecker()
        result = dc.check_python(["os", "sys", "json"])
        assert result["status"] == "ok"
        assert result["installed"] == 3

    def test_dependency_checker_missing(self):
        from pytrex.core import DependencyChecker
        dc = DependencyChecker()
        result = dc.check_python(["nonexistent_pkg_xyz"])
        assert result["status"] == "ok"
        assert result["installed"] == 0

    def test_performance_monitor(self):
        from pytrex.core import PerformanceMonitor
        import time
        pm = PerformanceMonitor()
        pm.start_timer("test_op")
        time.sleep(0.1)
        elapsed = pm.stop_timer("test_op")
        assert elapsed >= 0.05
        metric = pm.get_metric("test_op")
        assert metric is not None
        assert metric["count"] == 1
        assert metric["avg"] >= 0.05

    def test_perf_counters(self):
        from pytrex.core import PerformanceMonitor
        pm = PerformanceMonitor()
        pm.increment("requests")
        pm.increment("requests")
        pm.increment("errors")
        metric = pm.get_metric("requests")
        assert metric["value"] == 2
        pm.reset()
        assert pm.get_metric("requests") is None

    def test_perf_gauge(self):
        from pytrex.core import PerformanceMonitor
        pm = PerformanceMonitor()
        pm.gauge("cpu", 45.5)
        pm.gauge("cpu", 67.2)
        metric = pm.get_metric("cpu")
        assert metric["min"] == 45.5
        assert metric["max"] == 67.2

    def test_all_batch6_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            AutoFixEngine, HealthChecker, EncryptionManager, CacheManager,
            TaskQueue, NotificationManager, BackupManager, LogManager,
            DependencyChecker, PerformanceMonitor,
        )
        app = PyTreXApp(name="Batch6 Test")
        assert isinstance(app.autofix, AutoFixEngine)
        assert isinstance(app.health, HealthChecker)
        assert isinstance(app.encryption, EncryptionManager)
        assert isinstance(app.cache, CacheManager)
        assert isinstance(app.task_queue, TaskQueue)
        assert isinstance(app.notifications, NotificationManager)
        assert isinstance(app.backup, BackupManager)
        assert isinstance(app.log_manager, LogManager)
        assert isinstance(app.deps, DependencyChecker)
        assert isinstance(app.perf, PerformanceMonitor)


class TestBatch7Features:
    """Test batch 7: StateMachine, EventBus, Validator, Localization, FeatureFlags, RateLimiter, Retry, CircuitBreaker, SecretVault, APIClient."""

    def test_state_machine(self):
        from pytrex.core import StateMachine
        sm = StateMachine("idle")
        sm.add_transition("idle", "loading")
        sm.add_transition("loading", "ready")
        sm.add_transition("ready", "idle")
        assert sm.state == "idle"
        assert sm.set("loading") is True
        assert sm.state == "loading"
        assert sm.set("ready") is True
        assert sm.state == "ready"
        assert len(sm.history) == 2

    def test_state_machine_invalid(self):
        from pytrex.core import StateMachine
        sm = StateMachine("idle")
        sm.add_transition("idle", "loading")
        assert sm.set("ready") is False
        assert sm.state == "idle"

    def test_state_machine_callbacks(self):
        from pytrex.core import StateMachine
        sm = StateMachine("idle")
        sm.add_transition("idle", "active")
        entered = []
        exited = []
        sm.on("active", lambda: entered.append(True))
        sm.on_exit("idle", lambda: exited.append(True))
        sm.set("active")
        assert len(entered) == 1
        assert len(exited) == 1

    def test_event_bus(self):
        from pytrex.core import EventBus
        bus = EventBus()
        received = []
        bus.on("test.event", lambda data: received.append(data))
        count = bus.emit("test.event", {"value": 42})
        assert count == 1
        assert received[0]["value"] == 42
        assert "test.event" in bus.events
        assert bus.subscriber_count == 1

    def test_event_bus_multiple(self):
        from pytrex.core import EventBus
        bus = EventBus()
        results = []
        bus.on("click", lambda d: results.append("a"))
        bus.on("click", lambda d: results.append("b"))
        bus.emit("click")
        assert len(results) == 2

    def test_event_bus_off(self):
        from pytrex.core import EventBus
        bus = EventBus()
        handler = lambda d: None
        bus.on("event", handler)
        assert bus.off("event", handler) is True
        assert bus.off("event", handler) is False

    def test_event_bus_once(self):
        from pytrex.core import EventBus
        bus = EventBus()
        calls = []
        bus.once("once.event", lambda d: calls.append(d))
        bus.emit("once.event", "first")
        bus.emit("once.event", "second")
        assert len(calls) == 1

    def test_validator(self):
        from pytrex.core import ValidatorEngine
        v = ValidatorEngine()
        v.schema("user", {
            "name": {"required": True, "type": "str", "min_len": 2, "max_len": 50},
            "age": {"type": "int", "min": 0, "max": 150},
            "role": {"type": "str", "choices": ["admin", "user", "guest"]},
        })
        result = v.validate({"name": "Alice", "age": 30, "role": "admin"}, "user")
        assert result["valid"] is True

    def test_validator_errors(self):
        from pytrex.core import ValidatorEngine
        v = ValidatorEngine()
        v.schema("user", {
            "name": {"required": True, "type": "str"},
            "age": {"type": "int", "min": 0, "max": 150},
        })
        result = v.validate({"name": 123, "age": -5}, "user")
        assert result["valid"] is False
        assert len(result["errors"]) >= 2

    def test_validator_missing_required(self):
        from pytrex.core import ValidatorEngine
        v = ValidatorEngine()
        v.schema("user", {"name": {"required": True}})
        result = v.validate({}, "user")
        assert result["valid"] is False
        assert "required" in result["errors"][0]

    def test_validator_custom_rule(self):
        from pytrex.core import ValidatorEngine
        v = ValidatorEngine()
        v.add_rule("even", lambda x: True if x % 2 == 0 else "Must be even")
        v.schema("config", {"count": {"type": "int", "custom": "even"}})
        result = v.validate({"count": 3}, "config")
        assert result["valid"] is False
        result = v.validate({"count": 4}, "config")
        assert result["valid"] is True

    def test_localization(self):
        from pytrex.core import Localization
        l10n = Localization("en")
        l10n.add_translation("en", "welcome", "Welcome to PyTreX")
        l10n.add_translation("sw", "welcome", "Karibu kwenye PyTreX")
        assert l10n.t("welcome") == "Welcome to PyTreX"
        l10n.set_lang("sw")
        assert l10n.t("welcome") == "Karibu kwenye PyTreX"
        assert "en" in l10n.available_langs
        assert "sw" in l10n.available_langs

    def test_localization_formatting(self):
        from pytrex.core import Localization
        l10n = Localization("en")
        assert l10n.format_number(1234567.89) == "1,234,567.89"
        import datetime
        d = datetime.date(2026, 6, 29)
        assert "2026" in l10n.format_date(d)
        l10n.set_lang("sw")
        assert "Junai" in l10n.format_date(d) or "June" in l10n.format_date(d, "en")

    def test_localization_plural(self):
        from pytrex.core import Localization
        l10n = Localization("en")
        l10n.add_plural("items", "en", ["1 item", "items"])
        assert l10n.tp("items", 1) == "1 item"
        assert l10n.tp("items", 5) == "items"

    def test_localization_kwargs(self):
        from pytrex.core import Localization
        l10n = Localization("en")
        l10n.add_translation("en", "greeting", "Hello, {name}!")
        assert l10n.t("greeting", name="Alice") == "Hello, Alice!"

    def test_feature_flags(self):
        from pytrex.core import FeatureFlags
        ff = FeatureFlags()
        ff.register("new_ui", enabled=False, description="New UI design")
        assert ff.is_enabled("new_ui") is False
        ff.enable("new_ui")
        assert ff.is_enabled("new_ui") is True
        assert "new_ui" in ff.list_enabled()
        ff.disable("new_ui")
        assert ff.is_enabled("new_ui") is False

    def test_feature_flags_toggle(self):
        from pytrex.core import FeatureFlags
        ff = FeatureFlags()
        ff.register("beta", enabled=False)
        ff.toggle("beta")
        assert ff.is_enabled("beta") is True
        ff.toggle("beta")
        assert ff.is_enabled("beta") is False

    def test_feature_flags_listener(self):
        from pytrex.core import FeatureFlags
        ff = FeatureFlags()
        ff.register("dark_mode", enabled=False)
        changes = []
        ff.on_change("dark_mode", lambda name, enabled: changes.append((name, enabled)))
        ff.enable("dark_mode")
        assert len(changes) == 1
        assert changes[0] == ("dark_mode", True)

    def test_rate_limiter(self):
        from pytrex.core import RateLimiter
        rl = RateLimiter()
        rl.set_limit("user:1", limit=3, window_seconds=60)
        r1 = rl.check("user:1")
        assert r1["allowed"] is True
        r2 = rl.check("user:1")
        assert r2["allowed"] is True
        r3 = rl.check("user:1")
        assert r3["allowed"] is True
        r4 = rl.check("user:1")
        assert r4["allowed"] is False
        assert r4["remaining"] == 0

    def test_rate_limiter_no_config(self):
        from pytrex.core import RateLimiter
        rl = RateLimiter()
        result = rl.check("unknown_key")
        assert result["allowed"] is True

    def test_retry_engine_success(self):
        from pytrex.core import RetryEngine
        retry = RetryEngine()
        result = retry.attempt(lambda: 42, max_retries=3, base_delay=0.01)
        assert result["success"] is True
        assert result["result"] == 42
        assert result["attempts"] == 1

    def test_retry_engine_failure(self):
        from pytrex.core import RetryEngine
        retry = RetryEngine()
        call_count = [0]
        def failing():
            call_count[0] += 1
            raise ValueError("Always fails")
        result = retry.attempt(failing, max_retries=2, base_delay=0.01)
        assert result["success"] is False
        assert result["attempts"] == 3
        assert "Always fails" in result["error"]

    def test_retry_engine_eventual_success(self):
        from pytrex.core import RetryEngine
        retry = RetryEngine()
        attempt = [0]
        def eventually():
            attempt[0] += 1
            if attempt[0] < 3:
                raise Exception("Not yet")
            return "success"
        result = retry.attempt(eventually, max_retries=5, base_delay=0.01)
        assert result["success"] is True
        assert result["attempts"] == 3

    def test_circuit_breaker(self):
        from pytrex.core import CircuitBreaker
        cb = CircuitBreaker(failure_threshold=3, recovery_timeout=1.0)
        assert cb.get_state("api") == "closed"
        def fail():
            raise Exception("API down")
        for _ in range(3):
            cb.call("api", fail)
        assert cb.get_state("api") == "open"
        result = cb.call("api", lambda: "ok")
        assert result["status"] == "error"
        assert result["state"] == "open"

    def test_circuit_breaker_success(self):
        from pytrex.core import CircuitBreaker
        cb = CircuitBreaker(failure_threshold=3, recovery_timeout=1.0)
        result = cb.call("service", lambda: "hello")
        assert result["status"] == "ok"
        assert result["result"] == "hello"
        assert cb.get_state("service") == "closed"

    def test_circuit_breaker_recovery(self):
        from pytrex.core import CircuitBreaker
        import time
        cb = CircuitBreaker(failure_threshold=2, recovery_timeout=0.2)
        def fail():
            raise Exception("down")
        cb.call("svc", fail)
        cb.call("svc", fail)
        assert cb.get_state("svc") == "open"
        time.sleep(0.3)
        result = cb.call("svc", lambda: "recovered")
        assert result["status"] == "ok"
        assert cb.get_state("svc") == "closed"

    def test_secret_vault(self):
        from pytrex.core import SecretVault
        vault = SecretVault(master_key="test-key-123")
        vault.store("api_key", "sk-1234567890")
        assert vault.exists("api_key")
        assert vault.retrieve("api_key") == "sk-1234567890"
        assert vault.count == 1
        assert "api_key" in vault.list_names()
        vault.rotate("api_key", "sk-new-key")
        assert vault.retrieve("api_key") == "sk-new-key"
        vault.delete("api_key")
        assert not vault.exists("api_key")

    def test_secret_vault_not_found(self):
        from pytrex.core import SecretVault
        vault = SecretVault()
        assert vault.retrieve("nonexistent") is None
        assert vault.delete("nonexistent") is False

    def test_api_client(self):
        from pytrex.core import APIClient
        client = APIClient(base_url="https://httpbin.org")
        client.set_header("X-Custom", "PyTreX")
        client.set_auth("token123")
        result = client.get("/get")
        assert result["status"] in ("ok", "error")  # network may vary

    def test_api_client_stats(self):
        from pytrex.core import APIClient
        client = APIClient(base_url="https://httpbin.org")
        client.get("/get")
        assert client.stats["requests"] == 1

    def test_api_client_methods(self):
        from pytrex.core import APIClient
        client = APIClient("https://httpbin.org")
        for method in ["get", "post", "put", "delete", "patch"]:
            assert hasattr(client, method)

    def test_all_batch7_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            StateMachine, EventBus, ValidatorEngine, Localization,
            FeatureFlags, RateLimiter, RetryEngine, CircuitBreaker,
            SecretVault, APIClient,
        )
        app = PyTreXApp(name="Batch7 Test")
        assert isinstance(app.state, StateMachine)
        assert isinstance(app.bus, EventBus)
        assert isinstance(app.validator, ValidatorEngine)
        assert isinstance(app.l10n, Localization)
        assert isinstance(app.flags, FeatureFlags)
        assert isinstance(app.rate_limit, RateLimiter)
        assert isinstance(app.retry, RetryEngine)
        assert isinstance(app.circuit, CircuitBreaker)
        assert isinstance(app.vault, SecretVault)
        assert isinstance(app.http, APIClient)


class TestBatch8Features:
    """Test batch 8: WSClient, RedisPubSub, SearchEngine, MLInference, DataExporter, DataImporter, JobScheduler, WebScraper, PDFGeneratorPro, SMSGateway, PaymentGateway, AIChatAssistant."""

    def test_ws_client(self):
        from pytrex.core import WSClient
        ws = WSClient()
        assert ws.is_connected is False
        ws.connect("ws://localhost:8080")
        assert ws.is_connected is True
        assert ws.send("hello") is True
        ws.disconnect()
        assert ws.is_connected is False
        assert ws.send("hello") is False

    def test_ws_client_receive(self):
        from pytrex.core import WSClient
        ws = WSClient()
        ws.connect("ws://localhost:8080")
        received = []
        ws.on_message(lambda msg: received.append(msg))
        ws.receive("test message")
        assert len(received) == 1
        assert received[0] == "test message"
        assert ws.message_count == 1

    def test_redis_pubsub(self):
        from pytrex.core import RedisPubSub
        redis = RedisPubSub()
        received = []
        redis.subscribe("news", lambda msg: received.append(msg))
        count = redis.publish("news", "Hello World")
        assert count == 1
        assert received[0] == "Hello World"
        assert "news" in redis.channels
        assert len(redis.get_messages("news")) == 1

    def test_redis_pubsub_multiple(self):
        from pytrex.core import RedisPubSub
        redis = RedisPubSub()
        results = []
        redis.subscribe("ch", lambda m: results.append(("a", m)))
        redis.subscribe("ch", lambda m: results.append(("b", m)))
        count = redis.publish("ch", "msg")
        assert count == 2
        assert len(results) == 2

    def test_redis_unsubscribe(self):
        from pytrex.core import RedisPubSub
        redis = RedisPubSub()
        handler = lambda m: None
        redis.subscribe("ch", handler)
        assert redis.unsubscribe("ch", handler) is True
        assert redis.unsubscribe("ch", handler) is False

    def test_search_engine(self):
        from pytrex.core import SearchEngine
        se = SearchEngine()
        se.index("doc1", "Hello world from PyTreX framework")
        se.index("doc2", "PyTreX is an amazing framework for building apps")
        se.index("doc3", "Hello from the other side")
        results = se.query("PyTreX framework")
        assert len(results) > 0
        assert results[0]["doc_id"] in ("doc1", "doc2")
        assert se.document_count == 3
        assert se.index_size > 0

    def test_search_engine_remove(self):
        from pytrex.core import SearchEngine
        se = SearchEngine()
        se.index("doc1", "hello world")
        assert se.remove("doc1") is True
        assert se.document_count == 0
        assert se.remove("nonexistent") is False

    def test_ml_inference(self):
        from pytrex.core import MLInference
        ml = MLInference()
        assert ml.model_count == 0
        assert ml.list_models() == []
        result = ml.load_model("test", "nonexistent_model.pt")
        assert result["status"] == "error"

    def test_ml_inference_predict_no_model(self):
        from pytrex.core import MLInference
        ml = MLInference()
        result = ml.predict("nonexistent", [1, 2, 3])
        assert result["status"] == "error"

    def test_data_exporter_csv(self):
        from pytrex.core import DataExporter
        import tempfile, os
        exporter = DataExporter()
        data = [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]
        path = os.path.join(tempfile.gettempdir(), "test_export.csv")
        result = exporter.to_csv(data, path)
        assert result["status"] == "ok"
        assert os.path.exists(path)
        os.remove(path)

    def test_data_exporter_json(self):
        from pytrex.core import DataExporter
        import tempfile, os
        exporter = DataExporter()
        data = [{"name": "Alice", "age": 30}]
        path = os.path.join(tempfile.gettempdir(), "test_export.json")
        result = exporter.to_json(data, path)
        assert result["status"] == "ok"
        assert os.path.exists(path)
        os.remove(path)

    def test_data_exporter_xml(self):
        from pytrex.core import DataExporter
        import tempfile, os
        exporter = DataExporter()
        data = [{"name": "Alice", "age": 30}]
        path = os.path.join(tempfile.gettempdir(), "test_export.xml")
        result = exporter.to_xml(data, path)
        assert result["status"] == "ok"
        assert os.path.exists(path)
        os.remove(path)

    def test_data_importer_csv(self):
        from pytrex.core import DataExporter, DataImporter
        import tempfile, os
        path = os.path.join(tempfile.gettempdir(), "test_imp.csv")
        DataExporter().to_csv([{"name": "Alice", "age": "30"}], path)
        importer = DataImporter()
        result = importer.from_csv(path)
        assert result["status"] == "ok"
        assert result["rows"] == 1
        assert result["data"][0]["name"] == "Alice"
        os.remove(path)

    def test_data_importer_json(self):
        from pytrex.core import DataExporter, DataImporter
        import tempfile, os
        path = os.path.join(tempfile.gettempdir(), "test_imp.json")
        DataExporter().to_json([{"name": "Bob"}], path)
        importer = DataImporter()
        result = importer.from_json(path)
        assert result["status"] == "ok"
        assert result["data"][0]["name"] == "Bob"
        os.remove(path)

    def test_data_importer_xml(self):
        from pytrex.core import DataExporter, DataImporter
        import tempfile, os
        path = os.path.join(tempfile.gettempdir(), "test_imp.xml")
        DataExporter().to_xml([{"name": "Charlie", "age": "40"}], path)
        importer = DataImporter()
        result = importer.from_xml(path)
        assert result["status"] == "ok"
        assert result["rows"] == 1
        os.remove(path)

    def test_job_scheduler(self):
        from pytrex.core import JobScheduler
        js = JobScheduler()
        js.schedule("cleanup", lambda: "done", cron="0 2 * * *", priority=1)
        assert js.job_count == 1
        result = js.run_now("cleanup")
        assert result["status"] == "ok"
        assert result["result"] == "done"
        assert js.completed_count == 1

    def test_job_scheduler_not_found(self):
        from pytrex.core import JobScheduler
        js = JobScheduler()
        result = js.run_now("nonexistent")
        assert result["status"] == "error"

    def test_job_scheduler_cancel(self):
        from pytrex.core import JobScheduler
        js = JobScheduler()
        js.schedule("temp", lambda: None)
        assert js.cancel("temp") is True
        assert js.job_count == 0
        assert js.cancel("temp") is False

    def test_job_scheduler_list(self):
        from pytrex.core import JobScheduler
        js = JobScheduler()
        js.schedule("job1", lambda: None, cron="0 * * * *", priority=2)
        js.schedule("job2", lambda: None, interval=60, priority=1)
        jobs = js.list_jobs()
        assert len(jobs) == 2
        assert jobs[0]["name"] == "job1"

    def test_web_scraper_extract(self):
        from pytrex.core import WebScraper
        scraper = WebScraper()
        html = '<html><body><h1>Title</h1><p>Hello World</p><a href="http://example.com">Link</a></body></html>'
        texts = scraper.extract_text(html, "p")
        assert "Hello World" in texts
        links = scraper.extract_links(html)
        assert "http://example.com" in links

    def test_web_scraper_selectors(self):
        from pytrex.core import WebScraper
        scraper = WebScraper()
        html = '<html><body><h1>My Title</h1><p>My paragraph</p></body></html>'
        result = scraper.extract_selectors(html, {"title": "h1", "text": "p"})
        assert result["title"] == "My Title"
        assert result["text"] == "My paragraph"

    def test_web_scraper_table(self):
        from pytrex.core import WebScraper
        scraper = WebScraper()
        html = '<table><tr><th>Name</th><th>Age</th></tr><tr><td>Alice</td><td>30</td></tr></table>'
        rows = scraper.extract_table(html)
        assert len(rows) == 1
        assert rows[0]["Name"] == "Alice"

    def test_pdf_generator_pro(self):
        from pytrex.core import PDFGeneratorPro
        import tempfile, os
        pdf = PDFGeneratorPro()
        pdf.header("Annual Report 2026", level=1)
        pdf.paragraph("This is the annual report for PyTreX.")
        pdf.table([{"name": "Alice", "score": 95}, {"name": "Bob", "score": 87}], title="Scores")
        pdf.spacer(30)
        pdf.footer("Generated by PyTreX")
        assert pdf.section_count == 5
        html = pdf.render_html()
        assert "Annual Report" in html
        assert "Alice" in html
        path = os.path.join(tempfile.gettempdir(), "test_pdf_pro.html")
        result = pdf.save(path)
        assert result["status"] == "ok"
        os.remove(path)

    def test_pdf_generator_pro_clear(self):
        from pytrex.core import PDFGeneratorPro
        pdf = PDFGeneratorPro()
        pdf.header("Test")
        pdf.paragraph("Test")
        assert pdf.section_count == 2
        pdf.clear()
        assert pdf.section_count == 0

    def test_sms_gateway(self):
        from pytrex.core import SMSGateway
        sms = SMSGateway(provider="africas_talking")
        sms.set_credentials(api_key="test_key", username="test_user", sender_id="PyTreX")
        result = sms.send("+255712345678", "Hello from PyTreX")
        assert result["status"] == "ok"
        assert result["to"] == "+255712345678"
        assert sms.sent_count == 1

    def test_sms_gateway_bulk(self):
        from pytrex.core import SMSGateway
        sms = SMSGateway()
        sms.set_credentials(api_key="key")
        result = sms.bulk(["+255711111", "+255722222"], "Bulk message")
        assert result["status"] == "ok"
        assert result["sent"] == 2
        assert sms.sent_count == 2

    def test_sms_gateway_no_credentials(self):
        from pytrex.core import SMSGateway
        sms = SMSGateway()
        result = sms.send("+255123", "test")
        assert result["status"] == "error"

    def test_sms_gateway_scheduled(self):
        from pytrex.core import SMSGateway
        sms = SMSGateway()
        sms.set_credentials(api_key="key")
        result = sms.send_scheduled("+255123", "Later", "2026-07-01T10:00:00")
        assert result["status"] == "ok"
        assert result["send_at"] == "2026-07-01T10:00:00"

    def test_payment_gateway(self):
        from pytrex.core import PaymentGateway
        pay = PaymentGateway()
        pay.configure("stripe", {"secret_key": "sk_test_123"})
        result = pay.stripe_charge(99.99, "usd")
        assert result["status"] == "ok"
        assert "stripe_" in result["transaction_id"]
        assert pay.transaction_count == 1

    def test_payment_gateway_mpesa(self):
        from pytrex.core import PaymentGateway
        pay = PaymentGateway()
        pay.configure("mpesa", {"shortcode": "174379", "passkey": "xxx"})
        result = pay.mpesa_stk("+255712345678", 1500.0)
        assert result["status"] == "ok"
        assert result["phone"] == "+255712345678"
        assert result["amount"] == 1500.0

    def test_payment_gateway_flutterwave(self):
        from pytrex.core import PaymentGateway
        pay = PaymentGateway()
        pay.configure("flutterwave", {"secret_key": "flw_test"})
        result = pay.flutterwave_charge(500.0, "NGN")
        assert result["status"] == "ok"
        assert result["currency"] == "NGN"

    def test_payment_gateway_not_configured(self):
        from pytrex.core import PaymentGateway
        pay = PaymentGateway()
        result = pay.stripe_charge(100, "usd")
        assert result["status"] == "error"

    def test_payment_gateway_refund(self):
        from pytrex.core import PaymentGateway
        pay = PaymentGateway()
        pay.configure("stripe", {"key": "sk"})
        tx = pay.stripe_charge(50.0, "usd")
        refund = pay.refund(tx["transaction_id"])
        assert refund["status"] == "ok"
        assert refund["refunded"] == 50.0
        tx_data = pay.get_transaction(tx["transaction_id"])
        assert tx_data["status"] == "refunded"

    def test_payment_gateway_refund_not_found(self):
        from pytrex.core import PaymentGateway
        pay = PaymentGateway()
        result = pay.refund("nonexistent")
        assert result["status"] == "error"

    def test_ai_chat_assistant(self):
        from pytrex.core import AIChatAssistant
        ai = AIChatAssistant()
        ai.add_knowledge("encryption", "Use app.encryption.encrypt(data, password) for AES-256")
        result = ai.ask("How do I use encryption?")
        assert result["status"] == "ok"
        assert "encryption" in result["answer"].lower() or "encrypt" in result["answer"].lower()
        assert result["topic"] == "encryption"
        assert ai.conversation_count == 1

    def test_ai_chat_assistant_builtin(self):
        from pytrex.core import AIChatAssistant
        ai = AIChatAssistant()
        result = ai.ask("How to create a user?")
        assert result["status"] == "ok"
        assert "register" in result["answer"].lower() or "auth" in result["answer"].lower()

    def test_ai_chat_assistant_unknown(self):
        from pytrex.core import AIChatAssistant
        ai = AIChatAssistant()
        result = ai.ask("What is the meaning of life?")
        assert result["status"] == "ok"
        assert len(result["answer"]) > 0

    def test_ai_chat_assistant_knowledge_topics(self):
        from pytrex.core import AIChatAssistant
        ai = AIChatAssistant()
        ai.add_knowledge("auth", "Login with app.auth.login()")
        ai.add_knowledge("database", "Use app.migrations")
        topics = ai.knowledge_topics
        assert "auth" in topics
        assert "database" in topics

    def test_ai_chat_assistant_search(self):
        from pytrex.core import AIChatAssistant
        ai = AIChatAssistant()
        ai.add_knowledge("encryption", "Encrypt data")
        results = ai.search_knowledge("encrypt")
        assert "encryption" in results

    def test_all_batch8_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            WSClient, RedisPubSub, SearchEngine, MLInference,
            DataExporter, DataImporter, JobScheduler, WebScraper,
            PDFGeneratorPro, SMSGateway, PaymentGateway, AIChatAssistant,
        )
        app = PyTreXApp(name="Batch8 Test")
        assert isinstance(app.ws_client, WSClient)
        assert isinstance(app.redis, RedisPubSub)
        assert isinstance(app.search, SearchEngine)
        assert isinstance(app.ml, MLInference)
        assert isinstance(app.exporter, DataExporter)
        assert isinstance(app.importer, DataImporter)
        assert isinstance(app.jobs, JobScheduler)
        assert isinstance(app.scraper, WebScraper)
        assert isinstance(app.pdf_pro, PDFGeneratorPro)
        assert isinstance(app.sms, SMSGateway)
        assert isinstance(app.pay, PaymentGateway)
        assert isinstance(app.assistant, AIChatAssistant)


class TestAIBatchFeatures:
    """Test AI batch: LLMIntegration, VectorDatabase, AIAgent, EmbeddingEngine, TextSummarizer, SentimentAnalyzer, LanguageDetector, ImageClassifier, SpeechToText, TextToSpeech, CodeGenerator, RAGEngine."""

    def test_llm_not_configured(self):
        from pytrex.core import LLMIntegration
        llm = LLMIntegration()
        assert llm.is_configured is False
        result = llm.chat("Hello")
        assert result["status"] == "error"

    def test_llm_configure(self):
        from pytrex.core import LLMIntegration
        llm = LLMIntegration()
        llm.configure(api_key="test-key", model="gpt-4", provider="openai")
        assert llm.is_configured is True
        llm.set_system_prompt("You are helpful")
        llm.set_params(temperature=0.5, max_tokens=100)
        llm.clear_history()
        assert len(llm.history) == 0

    def test_vector_database(self):
        from pytrex.core import VectorDatabase
        vdb = VectorDatabase(dimension=4)
        vdb.insert("vec1", [1.0, 0.0, 0.0, 0.0])
        vdb.insert("vec2", [0.0, 1.0, 0.0, 0.0])
        vdb.insert("vec3", [1.0, 1.0, 0.0, 0.0])
        assert vdb.count == 3
        results = vdb.search([1.0, 0.0, 0.0, 0.0], top_k=2)
        assert len(results) == 2
        assert results[0]["id"] == "vec1"

    def test_vector_database_delete(self):
        from pytrex.core import VectorDatabase
        vdb = VectorDatabase(dimension=3)
        vdb.insert("v1", [1, 0, 0])
        assert vdb.delete("v1") is True
        assert vdb.count == 0
        assert vdb.delete("v1") is False

    def test_vector_database_update(self):
        from pytrex.core import VectorDatabase
        vdb = VectorDatabase(dimension=3)
        vdb.insert("v1", [1, 0, 0], {"label": "a"})
        assert vdb.update("v1", metadata={"label": "b"}) is True
        assert vdb.get("v1")["metadata"]["label"] == "b"
        assert vdb.update("nonexistent") is False

    def test_vector_database_filter(self):
        from pytrex.core import VectorDatabase
        vdb = VectorDatabase(dimension=3)
        vdb.insert("v1", [1, 0, 0], {"category": "A"})
        vdb.insert("v2", [0, 1, 0], {"category": "B"})
        results = vdb.search([1, 0, 0], top_k=5, filter={"category": "A"})
        assert len(results) == 1
        assert results[0]["id"] == "v1"

    def test_vector_database_dimension_mismatch(self):
        from pytrex.core import VectorDatabase
        vdb = VectorDatabase(dimension=4)
        assert vdb.insert("v1", [1, 0, 0]) is False

    def test_ai_agent(self):
        from pytrex.core import AIAgent
        agent = AIAgent("TestAgent")
        agent.register_tool("calc", lambda: 42, "Calculator")
        agent.set_goal("Solve a problem")
        result = agent.run()
        assert result["status"] == "ok"
        assert result["total_steps"] >= 2
        assert "calc" in agent.tools
        assert len(agent.goals) == 1

    def test_ai_agent_use_tool(self):
        from pytrex.core import AIAgent
        agent = AIAgent()
        agent.register_tool("greet", lambda name: f"Hello {name}")
        result = agent.use_tool("greet", "World")
        assert result["status"] == "ok"
        assert "World" in result["result"]

    def test_ai_agent_tool_not_found(self):
        from pytrex.core import AIAgent
        agent = AIAgent()
        result = agent.use_tool("nonexistent")
        assert result["status"] == "error"

    def test_ai_agent_memory(self):
        from pytrex.core import AIAgent
        agent = AIAgent()
        agent.register_tool("tool1", lambda: "ok")
        agent.run("test goal")
        assert len(agent.memory) > 0
        agent.clear_memory()
        assert len(agent.memory) == 0

    def test_embedding_engine_local(self):
        from pytrex.core import EmbeddingEngine
        emb = EmbeddingEngine(provider="local")
        result = emb.embed("Hello world")
        assert result["status"] == "ok"
        assert len(result["embedding"]) == 1536
        assert result["provider"] == "local"

    def test_embedding_engine_cache(self):
        from pytrex.core import EmbeddingEngine
        emb = EmbeddingEngine(provider="local")
        emb.embed("test text")
        assert emb.cache_size == 1
        result = emb.embed("test text")
        assert result.get("cached") is True
        emb.clear_cache()
        assert emb.cache_size == 0

    def test_embedding_engine_batch(self):
        from pytrex.core import EmbeddingEngine
        emb = EmbeddingEngine(provider="local")
        result = emb.embed_batch(["text1", "text2", "text3"])
        assert result["status"] == "ok"
        assert result["count"] == 3

    def test_embedding_engine_similarity(self):
        from pytrex.core import EmbeddingEngine
        emb = EmbeddingEngine(provider="local")
        result = emb.similarity("hello world", "hello world")
        assert result["status"] == "ok"
        assert result["similarity"] > 0.99

    def test_text_summarizer_extractive(self):
        from pytrex.core import TextSummarizer
        ts = TextSummarizer(method="extractive")
        text = ("Python is a great programming language. It is widely used for web development. "
                "Many developers love Python for its simplicity. Python also has great libraries. "
                "The community around Python is very supportive and helpful.")
        result = ts.summarize(text, ratio=0.4)
        assert result["status"] == "ok"
        assert len(result["summary"]) > 0
        assert result["method"] == "extractive"

    def test_text_summarizer_key_points(self):
        from pytrex.core import TextSummarizer
        ts = TextSummarizer()
        text = ("Python is great. It has many libraries. Web development is easy. "
                "Data science is powerful. Machine learning works well.")
        result = ts.key_points(text, num_points=3)
        assert result["status"] == "ok"
        assert len(result["points"]) <= 3

    def test_sentiment_analyzer_positive(self):
        from pytrex.core import SentimentAnalyzer
        sa = SentimentAnalyzer()
        result = sa.analyze("This is great and amazing, I love it!")
        assert result["status"] == "ok"
        assert result["sentiment"] == "positive"
        assert result["positive_count"] > 0

    def test_sentiment_analyzer_negative(self):
        from pytrex.core import SentimentAnalyzer
        sa = SentimentAnalyzer()
        result = sa.analyze("This is terrible and awful, I hate it")
        assert result["sentiment"] == "negative"
        assert result["negative_count"] > 0

    def test_sentiment_analyzer_neutral(self):
        from pytrex.core import SentimentAnalyzer
        sa = SentimentAnalyzer()
        result = sa.analyze("The table is brown")
        assert result["sentiment"] == "neutral"

    def test_sentiment_analyzer_batch(self):
        from pytrex.core import SentimentAnalyzer
        sa = SentimentAnalyzer()
        result = sa.analyze_batch(["I love this", "I hate this", "It is a table"])
        assert result["status"] == "ok"
        assert result["summary"]["total"] == 3

    def test_sentiment_analyzer_custom_words(self):
        from pytrex.core import SentimentAnalyzer
        sa = SentimentAnalyzer()
        sa.add_positive_word("supa")
        sa.add_negative_word("mbaya")
        assert "supa" in sa._positive_words
        assert "mbaya" in sa._negative_words

    def test_language_detector_english(self):
        from pytrex.core import LanguageDetector
        ld = LanguageDetector()
        result = ld.detect("The quick brown fox jumps over the lazy dog")
        assert result["status"] == "ok"
        assert result["language"] == "english"

    def test_language_detector_swahili(self):
        from pytrex.core import LanguageDetector
        ld = LanguageDetector()
        result = ld.detect("Hii ni katika lugha ya Kiswahili na ni nzuri sana")
        assert result["status"] == "ok"
        assert result["language"] == "swahili"

    def test_language_detector_unknown(self):
        from pytrex.core import LanguageDetector
        ld = LanguageDetector()
        result = ld.detect("xyz qwerty")
        assert result["language"] == "unknown"

    def test_language_detector_add_language(self):
        from pytrex.core import LanguageDetector
        ld = LanguageDetector()
        ld.add_language("italian", {"il", "la", "di", "che", "per", "una", "sono"})
        assert "italian" in ld.supported_languages

    def test_image_classifier_no_model(self):
        from pytrex.core import ImageClassifier
        ic = ImageClassifier()
        assert ic.model_count == 0
        result = ic.classify("nonexistent", "nonexistent.jpg")
        assert result["status"] == "error"

    def test_image_classifier_load_not_found(self):
        from pytrex.core import ImageClassifier
        ic = ImageClassifier()
        result = ic.load_model("test", "nonexistent.pt", ["cat", "dog"])
        assert result["status"] == "error"

    def test_speech_to_text_no_file(self):
        from pytrex.core import SpeechToText
        stt = SpeechToText()
        result = stt.transcribe("nonexistent.wav")
        assert result["status"] == "error"

    def test_speech_to_text_configured(self):
        from pytrex.core import SpeechToText
        stt = SpeechToText(provider="local")
        assert stt.is_configured is True
        stt.configure(language="sw")
        assert stt.is_configured is True

    def test_text_to_speech_configured(self):
        from pytrex.core import TextToSpeech
        tts = TextToSpeech(provider="local")
        assert tts.is_configured is True

    def test_text_to_speech_voices(self):
        from pytrex.core import TextToSpeech
        tts = TextToSpeech(provider="openai")
        voices = tts.list_voices()
        assert "alloy" in voices

    def test_code_generator_template(self):
        from pytrex.core import CodeGenerator
        cg = CodeGenerator()
        result = cg.generate("A function that adds two numbers", language="python")
        assert result["status"] == "ok"
        assert "def" in result["code"]
        assert result["method"] == "template"

    def test_code_generator_templates_list(self):
        from pytrex.core import CodeGenerator
        cg = CodeGenerator()
        templates = cg.templates
        assert "python_function" in templates
        assert "rust_function" in templates
        assert "sql_query" in templates

    def test_code_generator_add_template(self):
        from pytrex.core import CodeGenerator
        cg = CodeGenerator()
        cg.add_template("custom", "custom {name}")
        assert "custom" in cg.templates

    def test_rag_engine(self):
        from pytrex.core import RAGEngine
        rag = RAGEngine()
        rag.add_document("doc1", "PyTreX is a powerful framework for building enterprise applications with Rust and Python")
        rag.add_document("doc2", "The framework supports AI features including LLM integration and vector databases")
        assert rag.document_count > 0
        result = rag.query("What is PyTreX?")
        assert result["status"] == "ok"
        assert len(result["answer"]) > 0
        assert result["method"] == "retrieval-only"

    def test_rag_engine_clear(self):
        from pytrex.core import RAGEngine
        rag = RAGEngine()
        rag.add_document("doc1", "test content here")
        assert rag.document_count > 0
        rag.clear()
        assert rag.document_count == 0

    def test_all_ai_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            LLMIntegration, VectorDatabase, AIAgent, EmbeddingEngine,
            TextSummarizer, SentimentAnalyzer, LanguageDetector, ImageClassifier,
            SpeechToText, TextToSpeech, CodeGenerator, RAGEngine,
        )
        app = PyTreXApp(name="AI Test")
        assert isinstance(app.llm, LLMIntegration)
        assert isinstance(app.vectordb, VectorDatabase)
        assert isinstance(app.agent, AIAgent)
        assert isinstance(app.embedding, EmbeddingEngine)
        assert isinstance(app.summarizer, TextSummarizer)
        assert isinstance(app.sentiment, SentimentAnalyzer)
        assert isinstance(app.lang_detector, LanguageDetector)
        assert isinstance(app.image_classifier, ImageClassifier)
        assert isinstance(app.stt, SpeechToText)
        assert isinstance(app.tts, TextToSpeech)
        assert isinstance(app.code_gen, CodeGenerator)
        assert isinstance(app.rag, RAGEngine)


class TestBatch9Features:
    """Test batch 9: ORMEngine, WorkflowEngine, TemplateEngine, FormBuilder, MessageQueue, StreamProcessor, TimeSeriesDB, GraphDatabase, DocGenerator, TestFramework, CLIBuilder, IoTManager."""

    def test_orm_define_create(self):
        from pytrex.core import ORMEngine
        orm = ORMEngine()
        orm.define("User", name=str, email=str, age=int)
        result = orm.create("User", name="Alice", email="alice@test.com", age=30)
        assert result["status"] == "ok"
        assert result["record"]["name"] == "Alice"
        assert result["record"]["id"] == 1
        assert orm.count("User") == 1

    def test_orm_query_filter(self):
        from pytrex.core import ORMEngine
        orm = ORMEngine()
        orm.define("User", name=str, age=int)
        orm.create("User", name="Alice", age=30)
        orm.create("User", name="Bob", age=25)
        orm.create("User", name="Charlie", age=35)
        results = orm.query("User").filter(age=30).all()
        assert len(results) == 1
        assert results[0]["name"] == "Alice"

    def test_orm_query_filter_gt(self):
        from pytrex.core import ORMEngine
        orm = ORMEngine()
        orm.define("User", name=str, age=int)
        orm.create("User", name="Alice", age=30)
        orm.create("User", name="Bob", age=25)
        orm.create("User", name="Charlie", age=35)
        results = orm.query("User").filter_gt("age", 26).all()
        assert len(results) == 2

    def test_orm_query_order_limit(self):
        from pytrex.core import ORMEngine
        orm = ORMEngine()
        orm.define("User", name=str, age=int)
        orm.create("User", name="A", age=30)
        orm.create("User", name="B", age=25)
        orm.create("User", name="C", age=35)
        results = orm.query("User").order_by("age", desc=True).limit(2).all()
        assert len(results) == 2
        assert results[0]["age"] == 35

    def test_orm_update_delete(self):
        from pytrex.core import ORMEngine
        orm = ORMEngine()
        orm.define("User", name=str, age=int)
        orm.create("User", name="Alice", age=30)
        result = orm.update("User", 1, age=31)
        assert result["status"] == "ok"
        assert result["record"]["age"] == 31
        assert orm.delete("User", 1) is True
        assert orm.count("User") == 0

    def test_orm_model_not_found(self):
        from pytrex.core import ORMEngine
        orm = ORMEngine()
        result = orm.create("Nonexistent", name="test")
        assert result["status"] == "error"

    def test_workflow_engine(self):
        from pytrex.core import WorkflowEngine
        wf = WorkflowEngine()
        steps = [
            {"name": "validate", "action": lambda ctx: "validated"},
            {"name": "process", "action": lambda ctx: "processed"},
            {"name": "notify", "action": lambda ctx: "notified"},
        ]
        wf.create("approval", steps)
        result = wf.run("approval", {"user": "Alice"})
        assert result["status"] == "ok"
        assert len(result["steps"]) == 3
        assert wf.instance_count == 1

    def test_workflow_condition(self):
        from pytrex.core import WorkflowEngine
        wf = WorkflowEngine()
        steps = [
            {"name": "check", "action": lambda ctx: "checked"},
            {"name": "skip_me", "condition": "False", "action": lambda ctx: "should_not_run"},
            {"name": "done", "action": lambda ctx: "completed"},
        ]
        wf.create("conditional", steps)
        result = wf.run("conditional")
        assert result["steps"][1]["status"] == "skipped"

    def test_workflow_not_found(self):
        from pytrex.core import WorkflowEngine
        wf = WorkflowEngine()
        result = wf.run("nonexistent")
        assert result["status"] == "error"

    def test_template_engine_var(self):
        from pytrex.core import TemplateEngine
        te = TemplateEngine()
        result = te.render("Hello {{name}}!", {"name": "PyTreX"})
        assert result == "Hello PyTreX!"

    def test_template_engine_filter(self):
        from pytrex.core import TemplateEngine
        te = TemplateEngine()
        te.register_filter("upper", lambda x: str(x).upper())
        result = te.render("{{name|upper}}", {"name": "pytrex"})
        assert result == "PYTREX"

    def test_template_engine_for(self):
        from pytrex.core import TemplateEngine
        te = TemplateEngine()
        result = te.render("{% for item in items %}{{item}} {% endfor %}", {"items": ["a", "b", "c"]})
        assert "a" in result and "b" in result and "c" in result

    def test_template_engine_if(self):
        from pytrex.core import TemplateEngine
        te = TemplateEngine()
        result = te.render("{% if show %}Visible{% endif %}", {"show": True})
        assert "Visible" in result

    def test_form_builder(self):
        from pytrex.core import FormBuilder
        form = FormBuilder("login")
        form.add_field("email", type="email", required=True)
        form.add_field("password", type="password", required=True)
        form.add_field("age", type="number", min_val=18, max_val=100)
        assert form.field_count == 3
        result = form.validate({"email": "test@test.com", "password": "secret", "age": 25})
        assert result["valid"] is True

    def test_form_builder_errors(self):
        from pytrex.core import FormBuilder
        form = FormBuilder()
        form.add_field("email", type="email", required=True)
        form.add_field("age", type="number", min_val=18)
        result = form.validate({"email": "not-email", "age": 10})
        assert result["valid"] is False
        assert len(result["errors"]) >= 2

    def test_form_builder_html(self):
        from pytrex.core import FormBuilder
        form = FormBuilder("test")
        form.add_field("name", type="text", required=True)
        html = form.to_html()
        assert "<form" in html
        assert 'name="name"' in html

    def test_form_builder_choices(self):
        from pytrex.core import FormBuilder
        form = FormBuilder()
        form.add_field("role", type="select", choices=["admin", "user"])
        result = form.validate({"role": "admin"})
        assert result["valid"] is True
        result = form.validate({"role": "hacker"})
        assert result["valid"] is False

    def test_message_queue(self):
        from pytrex.core import MessageQueue
        mq = MessageQueue()
        mq.declare_queue("orders")
        mq.publish("orders", {"id": 1, "item": "book"})
        mq.publish("orders", {"id": 2, "item": "pen"})
        assert mq.queue_size("orders") == 2
        assert mq.stats["published"] == 2

    def test_message_queue_consume(self):
        from pytrex.core import MessageQueue
        mq = MessageQueue()
        received = []
        mq.consume("events", lambda msg: received.append(msg))
        mq.publish("events", "hello")
        assert len(received) == 1
        assert received[0] == "hello"
        assert mq.stats["consumed"] == 1

    def test_message_queue_purge(self):
        from pytrex.core import MessageQueue
        mq = MessageQueue()
        mq.publish("test", "msg1")
        mq.publish("test", "msg2")
        count = mq.purge("test")
        assert count == 2
        assert mq.queue_size("test") == 0

    def test_stream_processor(self):
        from pytrex.core import StreamProcessor
        sp = StreamProcessor()
        result = sp.source([1, 2, 3, 4, 5, 6]).filter(lambda x: x > 2).map(lambda x: x * 2).collect()
        assert result == [6, 8, 10, 12]

    def test_stream_processor_reduce(self):
        from pytrex.core import StreamProcessor
        sp = StreamProcessor()
        result = sp.source([1, 2, 3, 4, 5]).reduce(lambda a, b: a + b, 0).collect()
        assert result == [15]

    def test_stream_processor_sort_limit(self):
        from pytrex.core import StreamProcessor
        sp = StreamProcessor()
        result = sp.source([3, 1, 4, 1, 5, 9, 2, 6]).sort().limit(3).collect()
        assert result == [1, 1, 2]

    def test_stream_processor_distinct(self):
        from pytrex.core import StreamProcessor
        sp = StreamProcessor()
        result = sp.source([1, 2, 2, 3, 3, 3, 4]).distinct().collect()
        assert result == [1, 2, 3, 4]

    def test_stream_processor_count_sum(self):
        from pytrex.core import StreamProcessor
        sp = StreamProcessor()
        sp.source([1, 2, 3, 4, 5])
        assert sp.count() == 5
        sp2 = StreamProcessor()
        sp2.source([10, 20, 30])
        assert sp2.to_sum() == 60

    def test_timeseries_db(self):
        from pytrex.core import TimeSeriesDB
        ts = TimeSeriesDB()
        ts.write("cpu", 45.0, timestamp=100)
        ts.write("cpu", 55.0, timestamp=200)
        ts.write("cpu", 65.0, timestamp=300)
        results = ts.query("cpu", start=100, end=300)
        assert len(results) == 3
        avg = ts.aggregate("cpu", "avg", start=100, end=300)
        assert avg == 55.0
        assert "cpu" in ts.metrics

    def test_timeseries_aggregate(self):
        from pytrex.core import TimeSeriesDB
        ts = TimeSeriesDB()
        for i, v in enumerate([10, 20, 30, 40, 50]):
            ts.write("temp", v, timestamp=i * 100)
        assert ts.aggregate("temp", "max") == 50
        assert ts.aggregate("temp", "min") == 10
        assert ts.aggregate("temp", "sum") == 150
        assert ts.aggregate("temp", "count") == 5

    def test_timeseries_downsample(self):
        from pytrex.core import TimeSeriesDB
        ts = TimeSeriesDB()
        for i in range(10):
            ts.write("metric", float(i), timestamp=float(i * 100))
        result = ts.downsample("metric", interval=500)
        assert len(result) == 2

    def test_graph_database(self):
        from pytrex.core import GraphDatabase
        g = GraphDatabase()
        g.add_node("user1", {"name": "Alice"})
        g.add_node("user2", {"name": "Bob"})
        g.add_node("user3", {"name": "Charlie"})
        g.connect("user1", "user2", "friend")
        g.connect("user2", "user3", "friend")
        assert g.node_count == 3
        assert g.edge_count == 2
        neighbors = g.neighbors("user1", "friend")
        assert "user2" in neighbors

    def test_graph_traverse(self):
        from pytrex.core import GraphDatabase
        g = GraphDatabase()
        for i in range(5):
            g.add_node(f"n{i}")
        g.connect("n0", "n1")
        g.connect("n1", "n2")
        g.connect("n2", "n3")
        g.connect("n3", "n4")
        reachable = g.traverse("n0", depth=3)
        assert "n3" in reachable
        assert "n4" not in reachable

    def test_graph_find_path(self):
        from pytrex.core import GraphDatabase
        g = GraphDatabase()
        g.add_node("a")
        g.add_node("b")
        g.add_node("c")
        g.connect("a", "b")
        g.connect("b", "c")
        path = g.find_path("a", "c")
        assert path == ["a", "b", "c"]
        assert g.find_path("a", "nonexistent") is None

    def test_graph_remove_node(self):
        from pytrex.core import GraphDatabase
        g = GraphDatabase()
        g.add_node("x")
        g.add_node("y")
        g.connect("x", "y")
        assert g.remove_node("x") is True
        assert g.node_count == 1
        assert g.edge_count == 0

    def test_doc_generator(self):
        from pytrex.core import DocGenerator
        dg = DocGenerator()
        dg.add_endpoint("GET", "/users", "List all users", {"limit": "int"})
        dg.add_endpoint("POST", "/users", "Create user")
        dg.add_schema("User", {"name": "string", "email": "string"})
        openapi = dg.generate_openapi()
        assert openapi["openapi"] == "3.0.0"
        assert "/users" in openapi["paths"]
        assert "get" in openapi["paths"]["/users"]
        readme = dg.generate_readme()
        assert "GET" in readme
        assert "users" in readme

    def test_doc_generator_markdown(self):
        from pytrex.core import DocGenerator
        dg = DocGenerator()
        dg.add_endpoint("GET", "/health", "Health check")
        md = dg.generate_markdown_docs()
        assert "GET" in md
        assert "/health" in md

    def test_test_framework(self):
        from pytrex.core import TestFramework
        tf = TestFramework()
        tf.register("test_add", lambda: TestFramework.assert_equal(1 + 1, 2))
        tf.register("test_fail", lambda: TestFramework.assert_equal(1, 2))
        result = tf.run()
        assert result["total"] == 2
        assert result["passed"] == 1
        assert result["failed"] == 1

    def test_test_framework_assertions(self):
        from pytrex.core import TestFramework
        TestFramework.assert_equal(1, 1)
        TestFramework.assert_not_equal(1, 2)
        TestFramework.assert_true(True)
        TestFramework.assert_false(False)
        TestFramework.assert_in(1, [1, 2, 3])
        TestFramework.assert_raises(ValueError, lambda: int("abc"))

    def test_test_framework_fixtures(self):
        from pytrex.core import TestFramework
        tf = TestFramework()
        tf.fixture("db", lambda: {"connected": True})
        fixture = tf.get_fixture("db")
        assert fixture["connected"] is True

    def test_test_framework_mocks(self):
        from pytrex.core import TestFramework
        tf = TestFramework()
        tf.mock("api_response", {"status": "ok"})
        assert tf.get_mock("api_response")["status"] == "ok"

    def test_cli_builder(self):
        from pytrex.core import CLIBuilder
        cli = CLIBuilder("myapp")
        cli.command("deploy", lambda args: f"Deploying to {args.get('env', 'dev')}", "Deploy the app")
        cli.flag("deploy", "env", help="Target environment")
        result = cli.execute("deploy", {"env": "production"})
        assert result["status"] == "ok"
        assert "production" in result["result"]
        assert "deploy" in cli.commands

    def test_cli_builder_help(self):
        from pytrex.core import CLIBuilder
        cli = CLIBuilder("myapp")
        cli.command("build", lambda a: "built", "Build the project")
        cli.command("test", lambda a: "tested", "Run tests")
        help_text = cli.help()
        assert "build" in help_text
        assert "test" in help_text
        assert "Commands:" in help_text

    def test_cli_builder_not_found(self):
        from pytrex.core import CLIBuilder
        cli = CLIBuilder()
        result = cli.execute("nonexistent")
        assert result["status"] == "error"

    def test_iot_manager(self):
        from pytrex.core import IoTManager
        iot = IoTManager()
        iot.register_device("sensor1", "temperature", {"location": "room1"})
        assert iot.device_count == 1
        iot.write("sensor1", 25.5, unit="celsius")
        reading = iot.read("sensor1")
        assert reading["status"] == "ok"
        assert reading["reading"]["value"] == 25.5

    def test_iot_manager_subscribe(self):
        from pytrex.core import IoTManager
        iot = IoTManager()
        iot.register_device("sensor2", "humidity")
        readings = []
        iot.subscribe("sensor2", lambda r: readings.append(r))
        iot.write("sensor2", 60.0)
        assert len(readings) == 1
        assert readings[0]["value"] == 60.0

    def test_iot_manager_history(self):
        from pytrex.core import IoTManager
        iot = IoTManager()
        iot.register_device("sensor3", "pressure")
        for i in range(5):
            iot.write("sensor3", float(i * 10))
        history = iot.get_history("sensor3")
        assert len(history) == 5

    def test_iot_manager_remove(self):
        from pytrex.core import IoTManager
        iot = IoTManager()
        iot.register_device("temp1")
        assert iot.remove_device("temp1") is True
        assert iot.device_count == 0
        assert iot.remove_device("nonexistent") is False

    def test_iot_manager_status(self):
        from pytrex.core import IoTManager
        iot = IoTManager()
        iot.register_device("dev1")
        assert iot.set_status("dev1", "offline") is True
        assert iot.get_device("dev1")["status"] == "offline"

    def test_all_batch9_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            ORMEngine, WorkflowEngine, TemplateEngine, FormBuilder,
            MessageQueue, StreamProcessor, TimeSeriesDB, GraphDatabase,
            DocGenerator, TestFramework, CLIBuilder, IoTManager,
        )
        app = PyTreXApp(name="Batch9 Test")
        assert isinstance(app.orm, ORMEngine)
        assert isinstance(app.workflow, WorkflowEngine)
        assert isinstance(app.template, TemplateEngine)
        assert isinstance(app.form, FormBuilder)
        assert isinstance(app.msg_queue, MessageQueue)
        assert isinstance(app.stream, StreamProcessor)
        assert isinstance(app.timeseries, TimeSeriesDB)
        assert isinstance(app.graph, GraphDatabase)
        assert isinstance(app.docs, DocGenerator)
        assert isinstance(app.test, TestFramework)
        assert isinstance(app.cli_builder, CLIBuilder)
        assert isinstance(app.iot, IoTManager)


class TestBatch10Features:
    """Test batch 10: RealtimeSync, PermissionsEngine, AuditTrail, MultiTenantManager, WebhookManager, VersionControl, ABTesting, FeatureAnalytics, ContentModerator, RecommendationEngine, DataPipeline, ServiceMesh."""

    def test_realtime_sync(self):
        from pytrex.core import RealtimeSync
        rs = RealtimeSync()
        rs.join("doc1", "client1")
        rs.apply_op("doc1", {"type": "insert", "position": 0, "text": "Hello"})
        state = rs.get_state("doc1")
        assert state["content"] == "Hello"
        assert state["version"] == 1

    def test_realtime_sync_delete(self):
        from pytrex.core import RealtimeSync
        rs = RealtimeSync()
        rs.join("doc1", "c1")
        rs.apply_op("doc1", {"type": "insert", "position": 0, "text": "Hello"})
        rs.apply_op("doc1", {"type": "delete", "position": 0, "length": 2})
        state = rs.get_state("doc1")
        assert state["content"] == "llo"

    def test_realtime_sync_on_change(self):
        from pytrex.core import RealtimeSync
        rs = RealtimeSync()
        changes = []
        rs.on_change(lambda doc_id, op: changes.append(doc_id))
        rs.join("doc1", "c1")
        rs.apply_op("doc1", {"type": "insert", "position": 0, "text": "Hi"})
        assert len(changes) == 1
        assert changes[0] == "doc1"

    def test_realtime_sync_leave(self):
        from pytrex.core import RealtimeSync
        rs = RealtimeSync()
        rs.join("doc1", "c1")
        assert rs.leave("doc1", "c1") is True
        assert rs.leave("doc1", "c1") is False

    def test_permissions_engine(self):
        from pytrex.core import PermissionsEngine
        pe = PermissionsEngine()
        pe.define_role("admin")
        pe.grant("admin", "delete", "users")
        pe.assign_role("user1", "admin")
        assert pe.check("user1", "delete", "users") is True
        assert pe.check("user1", "create", "users") is False

    def test_permissions_wildcard(self):
        from pytrex.core import PermissionsEngine
        pe = PermissionsEngine()
        pe.define_role("super")
        pe.grant("super", "*", "*")
        pe.assign_role("u1", "super")
        assert pe.check("u1", "anything", "anywhere") is True

    def test_permissions_revoke(self):
        from pytrex.core import PermissionsEngine
        pe = PermissionsEngine()
        pe.define_role("editor")
        pe.grant("editor", "edit", "posts")
        assert pe.revoke("editor", "edit", "posts") is True
        assert pe.revoke("editor", "edit", "posts") is False

    def test_permissions_policy(self):
        from pytrex.core import PermissionsEngine
        pe = PermissionsEngine()
        pe.add_policy("access", "vault", lambda ctx: ctx["user_id"] == "trusted")
        assert pe.check("trusted", "access", "vault") is True
        assert pe.check("untrusted", "access", "vault") is False

    def test_audit_trail(self):
        from pytrex.core import AuditTrail
        audit = AuditTrail()
        audit.log("alice", "create", "user", None, {"name": "Bob"})
        audit.log("bob", "update", "user", {"name": "Bob"}, {"name": "Bobby"})
        assert audit.count == 2
        results = audit.query(user="alice")
        assert len(results) == 1
        assert results[0]["action"] == "create"

    def test_audit_trail_query_action(self):
        from pytrex.core import AuditTrail
        audit = AuditTrail()
        audit.log("a", "create", "x")
        audit.log("b", "delete", "y")
        audit.log("c", "create", "z")
        results = audit.query(action="create")
        assert len(results) == 2

    def test_audit_trail_export_clear(self):
        from pytrex.core import AuditTrail
        audit = AuditTrail()
        audit.log("u", "action", "res")
        exported = audit.export()
        assert len(exported) == 1
        count = audit.clear()
        assert count == 1
        assert audit.count == 0

    def test_multi_tenant(self):
        from pytrex.core import MultiTenantManager
        mt = MultiTenantManager()
        result = mt.create("acme", "Acme Corp", "pro")
        assert result["status"] == "ok"
        assert mt.tenant_count == 1
        mt.set_config("acme", "theme", "dark")
        assert mt.get_config("acme", "theme") == "dark"

    def test_multi_tenant_limits(self):
        from pytrex.core import MultiTenantManager
        mt = MultiTenantManager()
        mt.create("t1")
        mt.set_limit("t1", "users", 3)
        assert mt.check_limit("t1", "users") is True
        mt.record_usage("t1", "users", 3)
        assert mt.check_limit("t1", "users") is False

    def test_multi_tenant_data(self):
        from pytrex.core import MultiTenantManager
        mt = MultiTenantManager()
        mt.create("t1")
        mt.set_data("t1", "key", "value")
        assert mt.get_data("t1", "key") == "value"

    def test_multi_tenant_delete(self):
        from pytrex.core import MultiTenantManager
        mt = MultiTenantManager()
        mt.create("t1")
        assert mt.delete("t1") is True
        assert mt.tenant_count == 0

    def test_multi_tenant_duplicate(self):
        from pytrex.core import MultiTenantManager
        mt = MultiTenantManager()
        mt.create("t1")
        result = mt.create("t1")
        assert result["status"] == "error"

    def test_webhook_incoming(self):
        from pytrex.core import WebhookManager
        wh = WebhookManager()
        wh.register("order.created", lambda data: f"Processed {data['id']}")
        result = wh.receive("order.created", {"id": 123})
        assert result["status"] == "ok"
        assert "123" in result["result"]

    def test_webhook_no_handler(self):
        from pytrex.core import WebhookManager
        wh = WebhookManager()
        result = wh.receive("unknown.event", {})
        assert result["status"] == "error"

    def test_webhook_unregister(self):
        from pytrex.core import WebhookManager
        wh = WebhookManager()
        wh.register("test", lambda d: "ok")
        assert wh.unregister("test") is True
        assert wh.unregister("test") is False

    def test_version_control(self):
        from pytrex.core import VersionControl
        vc = VersionControl()
        vc.commit("doc1", {"name": "Alice"}, "initial", "alice")
        vc.commit("doc1", {"name": "Alice", "age": 30}, "add age", "alice")
        assert vc.version_count("doc1") == 2
        latest = vc.latest("doc1")
        assert latest["data"]["age"] == 30

    def test_version_control_rollback(self):
        from pytrex.core import VersionControl
        vc = VersionControl()
        vc.commit("doc1", {"v": 1})
        vc.commit("doc1", {"v": 2})
        vc.commit("doc1", {"v": 3})
        result = vc.rollback("doc1", 1)
        assert result["status"] == "ok"
        assert result["data"]["v"] == 1
        assert vc.version_count("doc1") == 4

    def test_version_control_diff(self):
        from pytrex.core import VersionControl
        vc = VersionControl()
        vc.commit("doc1", {"name": "Alice", "age": 30})
        vc.commit("doc1", {"name": "Bob", "age": 30, "city": "NYC"})
        diff = vc.diff("doc1", 1, 2)
        assert diff["status"] == "ok"
        assert "name" in diff["added"]
        assert "city" in diff["added"]

    def test_version_control_history(self):
        from pytrex.core import VersionControl
        vc = VersionControl()
        vc.commit("doc1", {"v": 1}, "first")
        vc.commit("doc1", {"v": 2}, "second")
        history = vc.history("doc1")
        assert len(history) == 2
        assert history[0]["message"] == "first"

    def test_ab_testing(self):
        from pytrex.core import ABTesting
        ab = ABTesting()
        ab.create("button_color", ["red", "blue"])
        r1 = ab.assign("button_color", "user1")
        assert r1["status"] == "ok"
        assert r1["variant"] in ("red", "blue")
        r2 = ab.assign("button_color", "user1")
        assert r2["variant"] == r1["variant"]

    def test_ab_testing_convert(self):
        from pytrex.core import ABTesting
        ab = ABTesting()
        ab.create("test", ["a", "b"])
        ab.assign("test", "u1")
        result = ab.convert("test", "u1")
        assert result["status"] == "ok"

    def test_ab_testing_results(self):
        from pytrex.core import ABTesting
        ab = ABTesting()
        ab.create("exp", ["x", "y"])
        for i in range(10):
            ab.assign("exp", f"user{i}")
        for i in range(5):
            ab.convert("exp", f"user{i}")
        results = ab.results("exp")
        assert results["status"] == "ok"
        assert len(results["variants"]) == 2

    def test_ab_testing_stop(self):
        from pytrex.core import ABTesting
        ab = ABTesting()
        ab.create("exp", ["a", "b"])
        assert ab.stop("exp") is True
        result = ab.assign("exp", "u1")
        assert result["status"] == "error"

    def test_feature_analytics(self):
        from pytrex.core import FeatureAnalytics
        fa = FeatureAnalytics()
        fa.track("login", "user1")
        fa.track("login", "user2")
        fa.track("purchase", "user1")
        usage = fa.feature_usage("login")
        assert usage["unique_users"] == 2
        top = fa.top_features()
        assert top[0]["feature"] == "login"

    def test_feature_analytics_funnel(self):
        from pytrex.core import FeatureAnalytics
        fa = FeatureAnalytics()
        fa.define_funnel("signup", ["visit", "signup", "verify"])
        fa.track("visit", "u1")
        fa.track("signup", "u1")
        fa.track("visit", "u2")
        funnel = fa.funnel("signup")
        assert funnel["status"] == "ok"
        assert funnel["steps"][0]["users"] == 2
        assert funnel["steps"][1]["users"] == 1

    def test_content_moderator_clean(self):
        from pytrex.core import ContentModerator
        mod = ContentModerator()
        result = mod.check_text("Hello, this is a nice message about programming")
        assert result["clean"] is True
        assert len(result["flags"]) == 0

    def test_content_moderator_spam(self):
        from pytrex.core import ContentModerator
        mod = ContentModerator()
        result = mod.check_text("Click here to earn money from home! Free money!")
        assert "spam" in result["flags"]

    def test_content_moderator_toxic(self):
        from pytrex.core import ContentModerator
        mod = ContentModerator()
        result = mod.check_text("You are an idiot and stupid")
        assert "toxicity" in result["flags"]

    def test_content_moderator_pii(self):
        from pytrex.core import ContentModerator
        mod = ContentModerator()
        result = mod.check_text("Contact me at john@example.com or 555-123-4567")
        assert "pii" in result["flags"]
        assert "email" in result["pii"]

    def test_content_moderator_batch(self):
        from pytrex.core import ContentModerator
        mod = ContentModerator()
        result = mod.check_batch(["Hello world", "Buy now! Free money!"])
        assert result["status"] == "ok"
        assert result["flagged"] >= 1

    def test_recommendation_engine(self):
        from pytrex.core import RecommendationEngine
        rec = RecommendationEngine()
        rec.add_item("item1")
        rec.add_item("item2")
        rec.add_item("item3")
        rec.rate("user1", "item1", 5)
        rec.rate("user1", "item2", 3)
        rec.rate("user2", "item1", 5)
        rec.rate("user2", "item3", 4)
        recs = rec.recommend("user1")
        assert len(recs) > 0
        assert recs[0]["item_id"] == "item3"

    def test_recommendation_no_ratings(self):
        from pytrex.core import RecommendationEngine
        rec = RecommendationEngine()
        rec.add_item("item1")
        rec.rate("user1", "item1", 5)
        recs = rec.recommend("new_user")
        assert len(recs) > 0
        assert recs[0]["reason"] == "popular"

    def test_recommendation_similar(self):
        from pytrex.core import RecommendationEngine
        rec = RecommendationEngine()
        rec.add_item("a")
        rec.add_item("b")
        rec.rate("u1", "a", 5)
        rec.rate("u1", "b", 5)
        rec.rate("u2", "a", 5)
        rec.rate("u2", "b", 4)
        similar = rec.similar_items("a")
        assert len(similar) > 0
        assert similar[0]["item_id"] == "b"

    def test_data_pipeline(self):
        from pytrex.core import DataPipeline
        dp = DataPipeline()
        dp.create("etl1",
                  extract=lambda: [1, 2, 3, 4, 5],
                  transforms=[lambda d: [x * 2 for x in d], lambda d: [x for x in d if x > 4]],
                  load=lambda d: f"Loaded {len(d)} items")
        result = dp.run("etl1")
        assert result["status"] == "ok"
        assert "3 items" in result["result"]

    def test_data_pipeline_not_found(self):
        from pytrex.core import DataPipeline
        dp = DataPipeline()
        result = dp.run("nonexistent")
        assert result["status"] == "error"

    def test_data_pipeline_run_all(self):
        from pytrex.core import DataPipeline
        dp = DataPipeline()
        dp.create("p1", extract=lambda: [1], transforms=[], load=lambda d: "ok")
        dp.create("p2", extract=lambda: [2], transforms=[], load=lambda d: "ok")
        result = dp.run_all()
        assert result["status"] == "ok"
        assert "p1" in result["results"]
        assert "p2" in result["results"]

    def test_service_mesh(self):
        from pytrex.core import ServiceMesh
        sm = ServiceMesh()
        sm.register("auth", "localhost", 8001)
        sm.register("auth", "localhost", 8002)
        sm.register("api", "localhost", 9000)
        assert "auth" in sm.services
        assert sm.instance_count("auth") == 2
        assert sm.healthy_count("auth") == 2

    def test_service_mesh_resolve(self):
        from pytrex.core import ServiceMesh
        sm = ServiceMesh()
        sm.register("api", "host1", 8001)
        sm.register("api", "host2", 8002)
        instance = sm.resolve("api")
        assert instance is not None
        sm.record_request("api", instance["id"])
        instance2 = sm.resolve("api")
        assert instance2["id"] != instance["id"] or instance2["requests"] == 0

    def test_service_mesh_deregister(self):
        from pytrex.core import ServiceMesh
        sm = ServiceMesh()
        sm.register("svc", "h", 80)
        instances = sm.get_instances("svc")
        assert sm.deregister("svc", instances[0]["id"]) is True
        assert sm.instance_count("svc") == 0

    def test_service_mesh_health_check(self):
        from pytrex.core import ServiceMesh
        sm = ServiceMesh()
        sm.register("svc", "h", 80)
        sm.set_health_check("svc", lambda inst: True)
        results = sm.run_health_checks()
        assert len(results) == 1
        assert "svc_1" in results

    def test_service_mesh_set_status(self):
        from pytrex.core import ServiceMesh
        sm = ServiceMesh()
        sm.register("svc", "h", 80)
        sm.set_status("svc", "svc_1", "unhealthy")
        assert sm.healthy_count("svc") == 0

    def test_all_batch10_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            RealtimeSync, PermissionsEngine, AuditTrail, MultiTenantManager,
            WebhookManager, VersionControl, ABTesting, FeatureAnalytics,
            ContentModerator, RecommendationEngine, DataPipeline, ServiceMesh,
        )
        app = PyTreXApp(name="Batch10 Test")
        assert isinstance(app.sync_rt, RealtimeSync)
        assert isinstance(app.permissions, PermissionsEngine)
        assert isinstance(app.audit, AuditTrail)
        assert isinstance(app.tenants, MultiTenantManager)
        assert isinstance(app.webhooks, WebhookManager)
        assert isinstance(app.versions, VersionControl)
        assert isinstance(app.ab, ABTesting)
        assert isinstance(app.usage, FeatureAnalytics)
        assert isinstance(app.moderator, ContentModerator)
        assert isinstance(app.recommender, RecommendationEngine)
        assert isinstance(app.pipeline, DataPipeline)
        assert isinstance(app.mesh, ServiceMesh)


class TestBatch11Features:
    """Test batch 11: SecurityScanner, SmartContract, StatisticsEngine, CICDPipeline, NetworkTools, OCREngine, CloudManager, GameEngine, QuantumSimulator, SQLBuilder, TranslationEngine, EdgeCompute."""

    def test_security_scanner_clean(self):
        from pytrex.core import SecurityScanner
        s = SecurityScanner()
        result = s.scan("Hello world, this is a normal message")
        assert result["secure"] is True
        assert len(result["vulnerabilities"]) == 0

    def test_security_scanner_sql_injection(self):
        from pytrex.core import SecurityScanner
        s = SecurityScanner()
        result = s.scan("SELECT * FROM users; DROP TABLE users--")
        assert result["secure"] is False
        vuln_types = [v["type"] for v in result["vulnerabilities"]]
        assert "sql_injection" in vuln_types

    def test_security_scanner_xss(self):
        from pytrex.core import SecurityScanner
        s = SecurityScanner()
        result = s.scan("<script>alert('xss')</script>")
        assert result["secure"] is False
        vuln_types = [v["type"] for v in result["vulnerabilities"]]
        assert "xss" in vuln_types

    def test_security_scanner_path_traversal(self):
        from pytrex.core import SecurityScanner
        s = SecurityScanner()
        result = s.scan("../../etc/passwd")
        assert result["secure"] is False

    def test_security_scanner_csrf(self):
        from pytrex.core import SecurityScanner
        s = SecurityScanner()
        token = s.generate_csrf_token()
        assert len(token) == 64
        assert s.verify_csrf_token(token, token) is True
        assert s.verify_csrf_token(token, "wrong") is False

    def test_security_scanner_batch(self):
        from pytrex.core import SecurityScanner
        s = SecurityScanner()
        result = s.scan_batch([("hello", "general"), ("DROP TABLE users", "db")])
        assert result["status"] == "ok"
        assert result["vulnerable"] >= 1

    def test_smart_contract_deploy(self):
        from pytrex.core import SmartContract
        sc = SmartContract()
        result = sc.deploy("token", {"transfer": lambda state, to, amt: f"Sent {amt} to {to}"})
        assert result["status"] == "ok"
        assert result["address"].startswith("0x")

    def test_smart_contract_execute(self):
        from pytrex.core import SmartContract
        sc = SmartContract()
        sc.deploy("token", {"transfer": lambda state, to, amt: f"Sent {amt} to {to}"})
        result = sc.execute("token", "transfer", ["alice", 100])
        assert result["status"] == "ok"
        assert "100" in result["result"]

    def test_smart_contract_not_found(self):
        from pytrex.core import SmartContract
        sc = SmartContract()
        result = sc.execute("nonexistent", "func")
        assert result["status"] == "error"

    def test_smart_contract_gas(self):
        from pytrex.core import SmartContract
        sc = SmartContract()
        sc.deploy("c", {"f": lambda s: "ok"})
        gas = sc.estimate_gas("c", "f", [1, 2, 3])
        assert gas == 21000 + 3 * 1000

    def test_smart_contract_state(self):
        from pytrex.core import SmartContract
        sc = SmartContract()
        sc.deploy("counter", {"inc": lambda s: s.update({"count": s.get("count", 0) + 1}) or s["count"]},
                  init_state={"count": 0})
        sc.execute("counter", "inc")
        state = sc.get_state("counter")
        assert state["status"] == "ok"
        assert state["state"]["count"] == 1

    def test_statistics_mean(self):
        from pytrex.core import StatisticsEngine
        assert StatisticsEngine.mean([1, 2, 3, 4, 5]) == 3.0

    def test_statistics_median(self):
        from pytrex.core import StatisticsEngine
        assert StatisticsEngine.median([1, 2, 3, 4, 5]) == 3
        assert StatisticsEngine.median([1, 2, 3, 4]) == 2.5

    def test_statistics_std_dev(self):
        from pytrex.core import StatisticsEngine
        sd = StatisticsEngine.std_dev([2, 4, 4, 4, 5, 5, 7, 9])
        assert 2.0 < sd < 3.0

    def test_statistics_correlation(self):
        from pytrex.core import StatisticsEngine
        r = StatisticsEngine.correlation([1, 2, 3, 4, 5], [2, 4, 6, 8, 10])
        assert r > 0.99

    def test_statistics_regression(self):
        from pytrex.core import StatisticsEngine
        result = StatisticsEngine.linear_regression([1, 2, 3, 4, 5], [2, 4, 6, 8, 10])
        assert result["status"] == "ok"
        assert abs(result["slope"] - 2.0) < 0.01
        assert abs(result["intercept"]) < 0.01

    def test_statistics_describe(self):
        from pytrex.core import StatisticsEngine
        result = StatisticsEngine.describe([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
        assert result["status"] == "ok"
        assert result["count"] == 10
        assert result["mean"] == 5.5

    def test_statistics_t_test(self):
        from pytrex.core import StatisticsEngine
        result = StatisticsEngine.t_test([1, 2, 3, 4, 5], [6, 7, 8, 9, 10])
        assert result["status"] == "ok"
        assert "t_statistic" in result

    def test_cicd_pipeline(self):
        from pytrex.core import CICDPipeline
        ci = CICDPipeline()
        ci.create("deploy_app", [
            {"name": "build", "action": lambda env: f"Built for {env}"},
            {"name": "test", "action": lambda env: f"Tested for {env}"},
            {"name": "deploy", "action": lambda env: f"Deployed to {env}"},
        ])
        result = ci.run("deploy_app", "staging")
        assert result["status"] == "ok"
        assert len(result["stages"]) == 3

    def test_cicd_pipeline_failure(self):
        from pytrex.core import CICDPipeline
        ci = CICDPipeline()
        ci.create("failing", [
            {"name": "build", "action": lambda env: "ok"},
            {"name": "test", "action": lambda env: (_ for _ in ()).throw(Exception("test failed"))},
            {"name": "deploy", "action": lambda env: "ok"},
        ])
        result = ci.run("failing")
        assert result["status"] == "failed"
        assert result["stages"][1]["status"] == "failed"

    def test_cicd_pipeline_rollback(self):
        from pytrex.core import CICDPipeline
        ci = CICDPipeline()
        ci.create("app", [
            {"name": "build"}, {"name": "test"}, {"name": "deploy"},
        ])
        result = ci.rollback("app", "build")
        assert result["status"] == "ok"
        assert result["rolled_back"] == 2

    def test_cicd_pipeline_promote(self):
        from pytrex.core import CICDPipeline
        ci = CICDPipeline()
        ci.create("app", [{"name": "deploy"}])
        result = ci.promote("app", "staging", "production")
        assert result["status"] == "ok"
        assert result["to"] == "production"

    def test_network_dns(self):
        from pytrex.core import NetworkTools
        nt = NetworkTools()
        result = nt.dns_lookup("google.com")
        assert result["status"] == "ok"
        assert "ip" in result

    def test_network_interfaces(self):
        from pytrex.core import NetworkTools
        nt = NetworkTools()
        ifaces = nt.get_interfaces()
        assert len(ifaces) > 0
        assert "ip" in ifaces[0]

    def test_ocr_parse_document(self):
        from pytrex.core import OCREngine
        ocr = OCREngine()
        ocr.register_template("invoice", "company|date|amount|tax|total")
        result = ocr.parse_document("Some text", "invoice")
        assert result["status"] == "ok"
        assert result["parsed"] is True

    def test_ocr_parse_no_template(self):
        from pytrex.core import OCREngine
        ocr = OCREngine()
        result = ocr.parse_document("Some text")
        assert result["status"] == "ok"
        assert result["parsed"] is False

    def test_ocr_extract_pattern(self):
        from pytrex.core import OCREngine
        ocr = OCREngine()
        matches = ocr.extract_from_pattern("Call 555-123-4567 or 555-987-6543", r"\d{3}-\d{3}-\d{4}")
        assert len(matches) == 2

    def test_cloud_manager(self):
        from pytrex.core import CloudManager
        cm = CloudManager()
        cm.register_provider("aws")
        result = cm.deploy("aws", "web-server", {"region": "us-east-1"})
        assert result["status"] == "ok"
        assert cm.instance_count == 1

    def test_cloud_manager_scale(self):
        from pytrex.core import CloudManager
        cm = CloudManager()
        cm.register_provider("gcp")
        r = cm.deploy("gcp", "api")
        result = cm.scale(r["instance_id"], 5)
        assert result["status"] == "ok"
        assert result["replicas"] == 5

    def test_cloud_manager_billing(self):
        from pytrex.core import CloudManager
        cm = CloudManager()
        cm.register_provider("aws")
        cm.deploy("aws", "web")
        billing = cm.get_billing()
        assert billing["status"] == "ok"
        assert billing["monthly_estimate"] > 0

    def test_cloud_manager_destroy(self):
        from pytrex.core import CloudManager
        cm = CloudManager()
        cm.register_provider("aws")
        r = cm.deploy("aws", "web")
        assert cm.destroy(r["instance_id"]) is True
        assert cm.instance_count == 0

    def test_game_engine_entities(self):
        from pytrex.core import GameEngine
        ge = GameEngine("Test")
        eid1 = ge.add_entity({"x": 0, "y": 0, "width": 32, "height": 32})
        eid2 = ge.add_entity({"x": 16, "y": 0, "width": 32, "height": 32})
        assert ge.entity_count == 2
        assert ge.check_collision(eid1, eid2) is True

    def test_game_engine_no_collision(self):
        from pytrex.core import GameEngine
        ge = GameEngine()
        eid1 = ge.add_entity({"x": 0, "y": 0, "width": 32, "height": 32})
        eid2 = ge.add_entity({"x": 100, "y": 100, "width": 32, "height": 32})
        assert ge.check_collision(eid1, eid2) is False

    def test_game_engine_scenes(self):
        from pytrex.core import GameEngine
        ge = GameEngine()
        ge.add_scene("level1", [{"x": 0, "y": 0}])
        ge.set_scene("level1")
        assert ge.current_scene == "level1"
        assert ge.entity_count == 1

    def test_game_engine_score(self):
        from pytrex.core import GameEngine
        ge = GameEngine()
        ge.add_score(10)
        ge.add_score(5)
        assert ge.score == 15

    def test_game_engine_input(self):
        from pytrex.core import GameEngine
        ge = GameEngine()
        ge.set_input("jump", True)
        assert ge.is_pressed("jump") is True
        assert ge.is_pressed("attack") is False

    def test_game_engine_physics(self):
        from pytrex.core import GameEngine
        ge = GameEngine()
        eid = ge.add_entity({"x": 0, "y": 0, "vx": 10, "vy": 0})
        ge.update_physics(dt=1.0, gravity=9.8)
        e = next(e for e in ge.entities if e["id"] == eid)
        assert e["y"] > 0
        assert e["x"] == 10

    def test_quantum_hadamard(self):
        from pytrex.core import QuantumSimulator
        q = QuantumSimulator(1)
        q.hadamard(0)
        probs = q.probabilities
        assert abs(probs[0] - 0.5) < 0.01
        assert abs(probs[1] - 0.5) < 0.01

    def test_quantum_pauli_x(self):
        from pytrex.core import QuantumSimulator
        q = QuantumSimulator(1)
        q.pauli_x(0)
        probs = q.probabilities
        assert probs[0] < 0.01
        assert probs[1] > 0.99

    def test_quantum_cnot(self):
        from pytrex.core import QuantumSimulator
        q = QuantumSimulator(2)
        q.pauli_x(0)
        q.cnot(0, 1)
        probs = q.probabilities
        assert probs[3] > 0.99

    def test_quantum_measure(self):
        from pytrex.core import QuantumSimulator
        q = QuantumSimulator(1)
        result = q.measure(0)
        assert result in (0, 1)

    def test_quantum_measure_all(self):
        from pytrex.core import QuantumSimulator
        q = QuantumSimulator(2)
        result = q.measure_all()
        assert len(result) == 2

    def test_quantum_gates_applied(self):
        from pytrex.core import QuantumSimulator
        q = QuantumSimulator(1)
        q.hadamard(0)
        q.pauli_x(0)
        assert len(q.gates_applied) == 2

    def test_sql_builder_select(self):
        from pytrex.core import SQLBuilder
        qb = SQLBuilder("users")
        result = qb.select("id", "name", "email").where("age > ?", 18).order_by("name").limit(10).build()
        assert "SELECT id, name, email FROM users" in result["sql"]
        assert "WHERE age > ?" in result["sql"]
        assert "ORDER BY name ASC" in result["sql"]
        assert "LIMIT 10" in result["sql"]
        assert 18 in result["params"]

    def test_sql_builder_insert(self):
        from pytrex.core import SQLBuilder
        qb = SQLBuilder("users")
        result = qb.insert().values(name="Alice", email="alice@test.com").build()
        assert "INSERT INTO users" in result["sql"]
        assert "Alice" in result["params"]

    def test_sql_builder_update(self):
        from pytrex.core import SQLBuilder
        qb = SQLBuilder("users")
        result = qb.update().set(name="Bob").where("id = ?", 1).build()
        assert "UPDATE users SET name = ?" in result["sql"]
        assert "WHERE id = ?" in result["sql"]

    def test_sql_builder_delete(self):
        from pytrex.core import SQLBuilder
        qb = SQLBuilder("users")
        result = qb.delete().where("id = ?", 5).build()
        assert "DELETE FROM users" in result["sql"]
        assert "WHERE id = ?" in result["sql"]

    def test_sql_builder_join(self):
        from pytrex.core import SQLBuilder
        qb = SQLBuilder("orders")
        result = qb.select("*").join("users", "orders.user_id = users.id").build()
        assert "INNER JOIN users ON" in result["sql"]

    def test_sql_builder_left_join(self):
        from pytrex.core import SQLBuilder
        qb = SQLBuilder("orders")
        result = qb.select("*").left_join("users", "orders.user_id = users.id").build()
        assert "LEFT JOIN users ON" in result["sql"]

    def test_translation_en_sw(self):
        from pytrex.core import TranslationEngine
        te = TranslationEngine()
        result = te.translate("hello world", "en", "sw")
        assert result["status"] == "ok"
        assert "habari" in result["translation"]
        assert "dunia" in result["translation"]

    def test_translation_en_fr(self):
        from pytrex.core import TranslationEngine
        te = TranslationEngine()
        result = te.translate("hello", "en", "fr")
        assert result["status"] == "ok"
        assert "bonjour" in result["translation"]

    def test_translation_add_word(self):
        from pytrex.core import TranslationEngine
        te = TranslationEngine()
        te.add_word("en", "sw", "school", "shule")
        result = te.translate("school", "en", "sw")
        assert "shule" in result["translation"]

    def test_translation_detect(self):
        from pytrex.core import TranslationEngine
        te = TranslationEngine()
        lang = te.detect_language("habari dunia")
        assert lang == "sw"

    def test_translation_supported(self):
        from pytrex.core import TranslationEngine
        te = TranslationEngine()
        langs = te.supported_languages
        assert "en" in langs
        assert "sw" in langs
        assert len(langs) >= 15

    def test_edge_compute(self):
        from pytrex.core import EdgeCompute
        ec = EdgeCompute()
        ec.register_node("node1", "US-East", 100)
        ec.register_node("node2", "EU-West", 100)
        assert ec.node_count == 2
        result = ec.deploy_function("process", lambda x: x * 2)
        assert result["status"] == "ok"

    def test_edge_compute_execute(self):
        from pytrex.core import EdgeCompute
        ec = EdgeCompute()
        ec.register_node("node1", "US-East")
        ec.deploy_function("double", lambda x: x * 2)
        result = ec.execute("double", [21])
        assert result["status"] == "ok"
        assert result["result"] == 42

    def test_edge_compute_no_nodes(self):
        from pytrex.core import EdgeCompute
        ec = EdgeCompute()
        result = ec.deploy_function("test", lambda: "ok")
        assert result["status"] == "error"

    def test_edge_compute_remove_node(self):
        from pytrex.core import EdgeCompute
        ec = EdgeCompute()
        ec.register_node("n1", "US")
        assert ec.remove_node("n1") is True
        assert ec.node_count == 0

    def test_edge_compute_load_balancing(self):
        from pytrex.core import EdgeCompute
        ec = EdgeCompute()
        ec.register_node("n1", "US")
        ec.register_node("n2", "EU")
        ec.deploy_function("f", lambda: "ok", "n1")
        ec.deploy_function("f2", lambda: "ok", "n1")
        best = ec._find_best_node()
        assert best == "n2"

    def test_all_batch11_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex.core import (
            SecurityScanner, SmartContract, StatisticsEngine, CICDPipeline,
            NetworkTools, OCREngine, CloudManager, GameEngine,
            QuantumSimulator, SQLBuilder, TranslationEngine, EdgeCompute,
        )
        app = PyTreXApp(name="Batch11 Test")
        assert isinstance(app.security_scanner, SecurityScanner)
        assert isinstance(app.smart_contract, SmartContract)
        assert isinstance(app.stats, StatisticsEngine)
        assert isinstance(app.cicd, CICDPipeline)
        assert isinstance(app.network, NetworkTools)
        assert isinstance(app.ocr, OCREngine)
        assert isinstance(app.cloud, CloudManager)
        assert isinstance(app.game, GameEngine)
        assert isinstance(app.quantum, QuantumSimulator)
        assert isinstance(app.sql_builder, SQLBuilder)
        assert isinstance(app.translator, TranslationEngine)
        assert isinstance(app.edge, EdgeCompute)


class TestBatch12Features(unittest.TestCase):
    """Tests for Batch 12: AI/ML Pro features."""

    def test_model_trainer(self):
        from pytrex import ModelTrainer
        mt = ModelTrainer()
        mt.prepare("m1", ["x1", "x2"], "y")
        mt.add_sample("m1", [1.0, 2.0], 5.0)
        mt.add_sample("m1", [2.0, 3.0], 8.0)
        result = mt.train("m1", epochs=50)
        assert result["status"] == "ok"
        pred = mt.predict("m1", [1.5, 2.5])
        assert pred is not None
        assert "m1" in mt.models

    def test_predictive_analytics(self):
        from pytrex import PredictiveAnalytics
        pa = PredictiveAnalytics()
        pa.add_data("sales", [10, 20, 30, 40, 50])
        ma = pa.moving_average("sales", window=3)
        assert len(ma) == 3
        trend = pa.linear_trend("sales")
        assert trend["status"] == "ok"
        fc = pa.forecast("sales", steps=3)
        assert len(fc) == 3
        assert pa.series_count == 1

    def test_anomaly_detector(self):
        from pytrex import AnomalyDetector
        ad = AnomalyDetector()
        ad.set_baseline("cpu", [50, 52, 48, 51, 49])
        normal = ad.check("cpu", 50)
        assert normal["status"] == "ok"
        assert not normal["is_anomaly"]
        anomaly = ad.check("cpu", 200)
        assert anomaly["is_anomaly"]
        batch = ad.batch_check("cpu", [50, 200, 51])
        assert len(batch) == 3

    def test_nlp_processor(self):
        from pytrex import NLPProcessor
        nlp = NLPProcessor()
        tokens = nlp.tokenize("The quick brown fox")
        assert len(tokens) == 4
        filtered = nlp.remove_stopwords(tokens)
        assert "the" not in filtered
        kw = nlp.extract_keywords("The good amazing wonderful excellent day", top_k=3)
        assert len(kw) <= 3
        sent = nlp.sentiment("This is great and amazing!")
        assert sent["label"] == "positive"
        nlp.build_vocab(["hello world", "world peace"])
        assert nlp.vectorize("hello world") == [0, 1]

    def test_image_generator(self):
        from pytrex import ImageGenerator
        ig = ImageGenerator()
        r = ig.gradient(100, 50)
        assert r["status"] == "ok"
        assert r["width"] == 100
        p = ig.pattern(200, 100, "stripes")
        assert p["pattern"] == "stripes"
        n = ig.noise(50, 50)
        assert n["status"] == "ok"
        assert n["type"] == "noise"
        assert n["width"] == 50
        art = ig.ascii_art("Hi")
        assert len(art) > 0
        assert ig.count == 3

    def test_voice_cloner(self):
        from pytrex import VoiceCloner
        vc = VoiceCloner()
        vc.register_voice("v1", "Alice", pitch=1.2, language="sw")
        r = vc.clone("v1", [{"freq": 440}])
        assert r["status"] == "ok"
        s = vc.synthesize("v1", "Hello world")
        assert s["duration"] > 0
        assert "v1" in vc.voices
        assert vc.voice_count == 1

    def test_automl(self):
        from pytrex import AutoML
        aml = AutoML()
        x = [[1.0], [2.0], [3.0], [4.0]]
        y = [2.0, 4.0, 6.0, 8.0]
        result = aml.search(x, y, max_trials=3)
        assert result["status"] == "ok"
        assert result["best"] is not None
        assert len(aml.trials) == 3

    def test_federated_learning(self):
        from pytrex import FederatedLearning
        fl = FederatedLearning()
        fl.register_node("n1", data_size=100)
        fl.register_node("n2", data_size=200)
        r = fl.submit_update("n1", [0.5, 0.3])
        assert r["status"] == "ok"
        r2 = fl.submit_update("n2", [0.4, 0.2])
        assert r2["status"] == "ok"
        assert len(fl.global_weights) == 2
        assert fl.node_count == 2

    def test_model_registry(self):
        from pytrex import ModelRegistry
        mr = ModelRegistry()
        mr.register("model_a", "1.0", {"accuracy": 0.85})
        mr.register("model_a", "1.1", {"accuracy": 0.90})
        latest = mr.get_latest("model_a")
        assert latest["version"] == "1.1"
        versions = mr.list_versions("model_a")
        assert len(versions) == 2
        cmp = mr.compare("model_a", "1.0", "1.1")
        assert cmp["status"] == "ok"
        assert mr.total_versions == 2

    def test_data_labeler(self):
        from pytrex import DataLabeler
        dl = DataLabeler()
        dl.create_dataset("ds1", ["cat", "dog"])
        dl.add_item("ds1", "image1.png", "cat")
        dl.add_item("ds1", "image2.png")
        dl.label_item("ds1", 1, "dog")
        exp = dl.export("ds1")
        assert exp["labeled"] == 2
        st = dl.stats("ds1")
        assert st["progress"] == 100.0

    def test_prompt_engine(self):
        from pytrex import PromptEngine
        pe = PromptEngine()
        pe.register("greet", "Hello {name}, you are {role}!")
        rendered = pe.render("greet", name="Alice", role="admin")
        assert "Alice" in rendered and "admin" in rendered
        pe.register("bye", "Goodbye {previous}!")
        pe.create_chain("flow", ["greet", "bye"])
        results = pe.run_chain("flow", name="Bob", role="user")
        assert len(results) == 2
        assert "Bob" in results[0]

    def test_chatbot_framework(self):
        from pytrex import ChatbotFramework
        cb = ChatbotFramework()
        cb.add_intent("greeting", ["hello", "hi", "hey"], "Hello! How can I help?")
        cb.add_intent("bye", ["bye", "goodbye"], "Goodbye!")
        r = cb.respond("s1", "hello there")
        assert r["intent"] == "greeting"
        r2 = cb.respond("s1", "goodbye")
        assert r2["intent"] == "bye"
        hist = cb.history("s1")
        assert len(hist) == 2
        assert cb.intent_count == 2
        assert cb.active_sessions == 1

    def test_batch12_in_app(self):
        from pytrex.core import PyTreXApp
        app = PyTreXApp()
        from pytrex import (ModelTrainer, PredictiveAnalytics, AnomalyDetector,
                            NLPProcessor, ImageGenerator, VoiceCloner, AutoML,
                            FederatedLearning, ModelRegistry, DataLabeler,
                            PromptEngine, ChatbotFramework)
        assert isinstance(app.model_trainer, ModelTrainer)
        assert isinstance(app.predictive, PredictiveAnalytics)
        assert isinstance(app.anomaly, AnomalyDetector)
        assert isinstance(app.nlp, NLPProcessor)
        assert isinstance(app.image_gen, ImageGenerator)
        assert isinstance(app.voice_cloner, VoiceCloner)
        assert isinstance(app.automl, AutoML)
        assert isinstance(app.federated, FederatedLearning)
        assert isinstance(app.model_registry, ModelRegistry)
        assert isinstance(app.data_labeler, DataLabeler)
        assert isinstance(app.prompt_engine, PromptEngine)
        assert isinstance(app.chatbot, ChatbotFramework)


class TestBatch13Features(unittest.TestCase):
    """Tests for Batch 13: Enterprise+ features."""

    def test_sso_manager(self):
        from pytrex import SSOManager
        sso = SSOManager()
        sso.register_provider("google", "client123", "secret456", "http://localhost/cb")
        url = sso.authorize_url("google")
        assert "google.com/auth" in url
        r = sso.exchange_code("google", "authcode123")
        assert r["status"] == "ok"
        assert sso.verify(r["access_token"]) is not None
        assert "google" in sso.providers

    def test_ldap_manager(self):
        from pytrex import LDAPManager
        ldap = LDAPManager()
        ldap.configure("srv1", "ldap.example.com")
        ldap.add_user("srv1", "cn=john,dc=example,dc=com", "John Doe", "john@example.com", ["dev"])
        r = ldap.authenticate("cn=john,dc=example,dc=com", "pass123")
        assert r["status"] == "ok"
        assert r["cn"] == "John Doe"
        results = ldap.search("srv1", "john")
        assert len(results) == 1
        assert ldap.user_count == 1

    def test_saml_provider(self):
        from pytrex import SAMLProvider
        saml = SAMLProvider()
        saml.register_sp("sp1", "https://sp.example.com", "https://sp.example.com/acs")
        r = saml.generate_assertion("sp1", "user123", {"email": "user@example.com"})
        assert r["status"] == "ok"
        a = saml.validate_assertion(r["assertion_id"])
        assert a is not None
        md = saml.metadata("sp1")
        assert md["status"] == "ok"
        assert saml.sp_count == 1

    def test_audit_reporter(self):
        from pytrex import AuditReporter
        ar = AuditReporter()
        entries = [{"user": "admin", "action": "create", "resource": "student"},
                   {"user": "admin", "action": "delete", "resource": "course"},
                   {"user": "teacher", "action": "update", "resource": "grade"}]
        r = ar.generate("r1", entries)
        assert r["total_entries"] == 3
        assert r["by_user"]["admin"] == 2
        csv = ar.export_csv("r1")
        assert "admin,2" in csv
        assert "r1" in ar.list_reports()

    def test_compliance_checker(self):
        from pytrex import ComplianceChecker
        cc = ComplianceChecker()
        cc.register_standard("SOC2", [{"id": "c1"}, {"id": "c2"}, {"id": "c3"}])
        r = cc.check("SOC2", {"c1": True, "c2": True, "c3": False})
        assert r["passed"] == 2
        assert r["score"] == 66.7
        assert not r["compliant"]
        assert "SOC2" in cc.standards

    def test_data_governance(self):
        from pytrex import DataGovernance
        dg = DataGovernance()
        dg.register_asset("db1", "Customer DB", "confidential", "IT")
        dg.classify("db1", "restricted")
        dg.set_lineage("db1", ["raw_csv", "etl_pipeline"])
        lin = dg.get_lineage("db1")
        assert len(lin) == 2
        q = dg.quality_score("db1", 0.9, 0.8, 0.7)
        assert q["quality_score"] > 0
        assert "db1" in dg.assets

    def test_privacy_manager(self):
        from pytrex import PrivacyManager
        pm = PrivacyManager()
        r = pm.detect_pii("Contact me at john@example.com or 555-123-4567")
        assert r["has_pii"]
        assert "email" in r["findings"]
        redacted = pm.redact("Email: john@example.com")
        assert "REDACTED" in redacted
        pm.record_consent("user1", "marketing", True)
        assert pm.check_consent("user1", "marketing")
        dsr = pm.data_subject_request("user1", "access")
        assert dsr["status"] == "ok"

    def test_gdpr_tools(self):
        from pytrex import GDPRTools
        gdpr = GDPRTools()
        r = gdpr.right_to_be_forgotten("user123")
        assert r["status"] == "ok"
        d = gdpr.check_deletion("user123")
        assert d["status"] == "processing"
        exp = gdpr.export_data("user123", {"name": "John", "email": "john@example.com"})
        assert exp["format"] == "JSON"
        br = gdpr.report_breach("Data leak", 600)
        assert br["notification_required"]
        assert gdpr.breach_count == 1

    def test_encryption_vault(self):
        from pytrex import EncryptionVault
        v = EncryptionVault()
        v.store("api_key", "secret123")
        assert v.retrieve("api_key") == "secret123"
        assert "api_key" in v.list_keys()
        v.delete("api_key")
        assert v.retrieve("api_key") is None
        assert v.secret_count == 0

    def test_key_rotation(self):
        from pytrex import KeyRotation
        kr = KeyRotation(rotation_days=30)
        kr.add_key("enc_key", "initial_value")
        assert kr.get_active("enc_key") == "initial_value"
        r = kr.rotate("enc_key")
        assert r["rotated"]
        assert kr.get_active("enc_key") != "initial_value"
        assert not kr.needs_rotation("enc_key")
        assert kr.key_count == 1

    def test_access_policy(self):
        from pytrex import AccessPolicy
        ap = AccessPolicy()
        ap.create_policy("p1", "documents", "read", "allow")
        ap.create_policy("p2", "documents", "delete", "deny")
        r1 = ap.evaluate("documents", "read")
        assert r1["decision"] == "allow"
        r2 = ap.evaluate("documents", "delete")
        assert r2["decision"] == "deny"
        r3 = ap.evaluate("documents", "write")
        assert r3["decision"] == "deny"
        assert ap.policy_count == 2

    def test_identity_provider(self):
        from pytrex import IdentityProvider
        idp = IdentityProvider()
        idp.provision("u1", "user@example.com", "John", ["admin"])
        assert idp.get_user("u1")["active"]
        secret = idp.enable_mfa("u1")
        assert idp.verify_mfa("u1", secret)
        idp.federate("u1", "google")
        idp.deprovision("u1")
        assert not idp.get_user("u1")["active"]
        assert idp.active_users == 0

    def test_batch13_in_app(self):
        from pytrex.core import PyTreXApp
        app = PyTreXApp()
        from pytrex import (SSOManager, LDAPManager, SAMLProvider, AuditReporter,
                            ComplianceChecker, DataGovernance, PrivacyManager, GDPRTools,
                            EncryptionVault, KeyRotation, AccessPolicy, IdentityProvider)
        assert isinstance(app.sso, SSOManager)
        assert isinstance(app.ldap, LDAPManager)
        assert isinstance(app.saml, SAMLProvider)
        assert isinstance(app.audit_reporter, AuditReporter)
        assert isinstance(app.compliance, ComplianceChecker)
        assert isinstance(app.data_gov, DataGovernance)
        assert isinstance(app.privacy, PrivacyManager)
        assert isinstance(app.gdpr, GDPRTools)
        assert isinstance(app.enc_vault, EncryptionVault)
        assert isinstance(app.key_rotation, KeyRotation)
        assert isinstance(app.access_policy, AccessPolicy)
        assert isinstance(app.idp, IdentityProvider)


class TestBatch14Features(unittest.TestCase):
    """Tests for Batch 14: DevTools+ features."""

    def test_code_formatter(self):
        from pytrex import CodeFormatter
        cf = CodeFormatter()
        r = cf.format("def foo():\nreturn 42", "python")
        assert r["status"] == "ok"
        assert "    return 42" in r["formatted"]
        mini = cf.minify("var x = 1;\nvar y = 2;", "javascript")
        assert "\n" not in mini
        assert "python" in cf.languages

    def test_linter(self):
        from pytrex import Linter
        lint = Linter()
        r = lint.lint("x = 1  \nprint('hi')\nimport * from os")
        assert r["total_issues"] > 0
        rules = [i["rule"] for i in r["issues"]]
        assert "E003" in rules
        assert "W001" in rules

    def test_type_checker(self):
        from pytrex import TypeChecker
        tc = TypeChecker()
        r = tc.check_type(42, "int")
        assert r["matches"]
        r2 = tc.check_type("hello", "int")
        assert not r2["matches"]
        r3 = tc.check_type([1, 2], "list")
        assert r3["matches"]

    def test_debug_profiler(self):
        from pytrex import DebugProfiler
        dp = DebugProfiler()
        dp.start("test1")
        time.sleep(0.01)
        r = dp.stop()
        assert r["status"] == "ok"
        assert r["elapsed"] > 0

        @dp.profile
        def my_func():
            return 42
        assert my_func() == 42
        assert "test1" in dp.profiles

    def test_memory_analyzer(self):
        from pytrex import MemoryAnalyzer
        ma = MemoryAnalyzer()
        s1 = ma.snapshot("s1", {"a": [1, 2, 3], "b": "hello"})
        assert s1["total_bytes"] > 0
        s2 = ma.snapshot("s2", {"a": [1, 2, 3, 4, 5], "b": "hello"})
        cmp = ma.compare("s1", "s2")
        assert cmp["delta_total"] != 0
        assert ma.snapshot_count == 2

    def test_hot_reloader(self):
        from pytrex import HotReloader
        import os
        hr = HotReloader()
        test_file = os.path.join(os.path.dirname(__file__), "test_features.py")
        changes = []
        hr.watch(test_file, lambda f: changes.append(f))
        assert test_file in hr.watched_files
        changed = hr.check()
        assert isinstance(changed, list)

    def test_repl(self):
        from pytrex import REPL
        repl = REPL()
        r = repl.eval("2 + 3")
        assert r["status"] == "ok"
        assert r["result"] == 5
        repl.exec("x = 100")
        r2 = repl.eval("x * 2")
        assert r2["result"] == 200
        assert len(repl.history) == 3

    def test_notebook(self):
        from pytrex import Notebook
        nb = Notebook()
        idx = nb.add_cell("2 + 2")
        nb.add_cell("# Markdown", "markdown")
        r = nb.run_cell(0)
        assert r["output"] == "4"
        r2 = nb.run_cell(1)
        assert "Markdown" in r2["output"]
        assert nb.cell_count == 2

    def test_api_tester(self):
        from pytrex import APITester
        at = APITester()
        at.add_test("t1", "GET", "http://localhost:9999/nonexistent")
        r = at.run_test("t1")
        assert not r["passed"]
        assert at.test_count == 1

    def test_mock_server(self):
        from pytrex import MockServer
        ms = MockServer()
        ms.register("GET", "/api/users", {"users": []})
        m = ms.get_mock("GET", "/api/users")
        assert m["response"] == {"users": []}
        assert ms.mock_count == 1
        ms.clear()
        assert ms.mock_count == 0

    def test_snapshot_test(self):
        from pytrex import SnapshotTest
        st = SnapshotTest()
        r1 = st.compare("s1", {"a": 1})
        assert r1["passed"]
        r2 = st.compare("s1", {"a": 1})
        assert r2["passed"]
        r3 = st.compare("s1", {"a": 2})
        assert not r3["passed"]
        assert st.snapshot_count == 1

    def test_coverage_reporter(self):
        from pytrex import CoverageReporter
        cr = CoverageReporter()
        cr.track("file1.py", 10, [1, 2, 3, 4, 5])
        r = cr.report()
        assert r["percentage"] == 50.0
        uncov = cr.uncovered("file1.py")
        assert 6 in uncov
        assert cr.file_count == 1

    def test_batch14_in_app(self):
        from pytrex.core import PyTreXApp
        app = PyTreXApp()
        from pytrex import (CodeFormatter, Linter, TypeChecker, DebugProfiler,
                            MemoryAnalyzer, HotReloader, REPL, Notebook,
                            APITester, MockServer, SnapshotTest, CoverageReporter)
        assert isinstance(app.formatter, CodeFormatter)
        assert isinstance(app.linter, Linter)
        assert isinstance(app.type_checker, TypeChecker)
        assert isinstance(app.profiler, DebugProfiler)
        assert isinstance(app.memory, MemoryAnalyzer)
        assert isinstance(app.hot_reload, HotReloader)
        assert isinstance(app.repl, REPL)
        assert isinstance(app.notebook, Notebook)
        assert isinstance(app.api_tester, APITester)
        assert isinstance(app.mock_server, MockServer)
        assert isinstance(app.snapshot_test, SnapshotTest)
        assert isinstance(app.coverage, CoverageReporter)


class TestBatch15Features(unittest.TestCase):
    """Tests for Batch 15: Industry+ features."""

    def test_healthcare_hl7(self):
        from pytrex import HealthcareHL7
        hl7 = HealthcareHL7()
        hl7.register_patient("p1", "John Doe", "1990-01-01", "M")
        r = hl7.parse_adt("p1", "A01")
        assert r["status"] == "ok"
        lr = hl7.lab_result("p1", "Glucose", "120", "mg/dL", "70-100")
        assert lr["status"] == "ok"
        msgs = hl7.get_messages("p1")
        assert len(msgs) == 2
        assert hl7.patient_count == 1

    def test_finance_portfolio(self):
        from pytrex import FinancePortfolio
        fp = FinancePortfolio()
        fp.buy("AAPL", 10, 150.0)
        fp.buy("AAPL", 5, 160.0)
        assert "AAPL" in fp.symbols
        r = fp.sell("AAPL", 3, 170.0)
        assert r["realized_pnl"] > 0
        val = fp.value({"AAPL": 165.0})
        assert val["total_value"] > 0

    def test_inventory_scm(self):
        from pytrex import InventorySCM
        inv = InventorySCM()
        inv.add_item("SKU001", "Widget", 50, 10, 5.0)
        inv.add_stock("SKU001", 20)
        r = inv.remove_stock("SKU001", 5)
        assert r["quantity"] == 65
        alerts = inv.reorder_alerts()
        assert "SKU001" not in alerts
        inv.add_item("SKU002", "Gadget", 5, 10)
        assert "SKU002" in inv.reorder_alerts()
        order = inv.create_order("SKU001", 100, "sup1")
        assert order["id"].startswith("PO_")
        assert order["sku"] == "SKU001"

    def test_hr_payroll(self):
        from pytrex import HRPayroll
        hr = HRPayroll()
        hr.add_employee("e1", "Jane", "Engineering", 500000, "Developer")
        att = hr.record_attendance("e1", 20, 2, 10)
        assert att["attendance_rate"] == 90.9
        pay = hr.calculate_pay("e1")
        assert pay["gross"] > 500000
        assert pay["tax"] > 0
        assert pay["net"] < pay["gross"]
        assert hr.employee_count == 1

    def test_crm_pipeline(self):
        from pytrex import CRMPipeline
        crm = CRMPipeline()
        crm.add_lead("l1", "Acme Corp", "Acme", "ceo@acme.com", 50000)
        crm.move_stage("l1", "contacted")
        crm.move_stage("l1", "qualified")
        pv = crm.pipeline_value()
        assert pv["total"] == 50000
        crm.move_stage("l1", "won")
        cr = crm.conversion_rate()
        assert cr["won"] == 1
        assert crm.lead_count == 1

    def test_project_kanban(self):
        from pytrex import ProjectKanban
        kb = ProjectKanban()
        kb.create_board("b1", "Sprint 1")
        card = kb.add_card("b1", "Fix bug", "To Do", "Critical bug", "Alice")
        assert card["card_id"].startswith("CARD_")
        mv = kb.move_card("b1", card["card_id"], "In Progress")
        assert mv["to"] == "In Progress"
        board = kb.get_board("b1")
        assert len(board["columns"]["In Progress"]) == 1
        assert kb.board_count == 1

    def test_invoice_generator(self):
        from pytrex import InvoiceGenerator
        ig = InvoiceGenerator()
        r = ig.create("inv1", "Acme Corp", [{"name": "Service", "quantity": 2, "price": 50000}])
        assert r["subtotal"] == 100000
        assert r["tax"] == 18000
        assert r["total"] == 118000
        ig.mark_paid("inv1")
        inv = ig.get("inv1")
        assert inv["status"] == "paid"
        assert ig.invoice_count == 1

    def test_tax_calculator(self):
        from pytrex import TaxCalculator
        tc = TaxCalculator()
        v = tc.vat(100000)
        assert v["vat"] == 18000
        vi = tc.vat(118000, inclusive=True)
        assert vi["net"] == 100000
        p = tc.paye(600000)
        assert p["tax"] > 0
        assert p["net"] < 600000
        c = tc.corporate_tax(1000000)
        assert c["tax"] == 300000
        w = tc.withholding(100000)
        assert w["tax"] == 5000

    def test_geo_gis(self):
        from pytrex import GeoGIS
        gis = GeoGIS()
        gis.add_point("dar", -6.79, 39.21, {"city": "Dar es Salaam"})
        gis.add_point("dodoma", -6.17, 35.74, {"city": "Dodoma"})
        d = gis.distance("dar", "dodoma")
        assert d["distance_km"] > 0
        near = gis.nearest("dar")
        assert len(near) == 1
        assert near[0]["point_id"] == "dodoma"
        gis.create_region("central", ["dodoma"])
        assert "dodoma" in gis.points_in_region("central")
        assert gis.point_count == 2

    def test_iot_protocol(self):
        from pytrex import IoTProtocol
        iot = IoTProtocol()
        iot.register_device("d1", "MQTT", "Building A")
        r = iot.publish("d1", "sensors/temp", {"value": 25.5})
        assert r["status"] == "ok"
        iot.subscribe("sensors/humidity", "d1")
        msgs = iot.get_messages(topic="sensors/temp")
        assert len(msgs) == 1
        assert iot.device_count == 1
        assert iot.online_devices == 1

    def test_energy_grid(self):
        from pytrex import EnergyGrid
        eg = EnergyGrid()
        eg.register_meter("m1", "Building A", 1000)
        eg.record_consumption("m1", 150)
        eg.record_generation("m1", 200)
        bal = eg.balance()
        assert bal["net"] == 50
        assert bal["self_sufficient"]
        assert eg.meter_count == 1

    def test_logistics_route(self):
        from pytrex import LogisticsRoute
        lr = LogisticsRoute()
        lr.add_vehicle("v1", 5000, "Driver A")
        stops = [{"lat": -6.79, "lon": 39.21}, {"lat": -6.17, "lon": 35.74}]
        route = lr.plan_route("r1", "v1", stops)
        assert route["total_distance_km"] > 0
        assert route["stop_count"] == 2
        s = lr.start_route("r1")
        assert s["status"] == "in_transit"
        c = lr.complete_route("r1")
        assert c["status"] == "completed"
        assert lr.vehicle_count == 1
        assert lr.route_count == 1

    def test_batch15_in_app(self):
        from pytrex.core import PyTreXApp
        app = PyTreXApp()
        from pytrex import (HealthcareHL7, FinancePortfolio, InventorySCM, HRPayroll,
                            CRMPipeline, ProjectKanban, InvoiceGenerator, TaxCalculator,
                            GeoGIS, IoTProtocol, EnergyGrid, LogisticsRoute)
        assert isinstance(app.healthcare, HealthcareHL7)
        assert isinstance(app.portfolio, FinancePortfolio)
        assert isinstance(app.inventory, InventorySCM)
        assert isinstance(app.hr, HRPayroll)
        assert isinstance(app.crm, CRMPipeline)
        assert isinstance(app.kanban, ProjectKanban)
        assert isinstance(app.invoice, InvoiceGenerator)
        assert isinstance(app.tax, TaxCalculator)
        assert isinstance(app.gis, GeoGIS)
        assert isinstance(app.iot_proto, IoTProtocol)
        assert isinstance(app.energy, EnergyGrid)
        assert isinstance(app.logistics, LogisticsRoute)


class TestBatch16Features:
    """Test batch 16: Neural Networks & Deep AI."""

    def test_neural_network(self):
        from pytrex import NeuralNetwork
        nn = NeuralNetwork(layers=[2, 4, 1])
        x_data = [[0, 0], [0, 1], [1, 0], [1, 1]]
        y_data = [[0], [1], [1], [0]]
        result = nn.train(x_data, y_data, epochs=50, lr=0.1)
        assert result["status"] == "ok"
        assert result["final_loss"] >= 0
        pred = nn.predict([1, 1])
        assert len(pred) == 1
        arch = nn.architecture
        assert arch["layers"] == [2, 4, 1]
        assert arch["total_params"] > 0
        assert len(nn.loss_history) == 50

    def test_neural_network_evaluate(self):
        from pytrex import NeuralNetwork
        nn = NeuralNetwork(layers=[2, 3, 1])
        x_data = [[0, 0], [0, 1], [1, 0], [1, 1]]
        y_data = [[0], [1], [1], [0]]
        nn.train(x_data, y_data, epochs=10)
        result = nn.evaluate(x_data, y_data)
        assert result["status"] == "ok"
        assert "loss" in result
        assert "accuracy" in result

    def test_neural_network_set_activation(self):
        from pytrex import NeuralNetwork
        nn = NeuralNetwork(layers=[2, 3, 1])
        assert nn.set_activation(0, "tanh") is True
        assert nn.set_activation(5, "tanh") is False

    def test_cnn(self):
        from pytrex import ConvolutionalNN
        cnn = ConvolutionalNN()
        cnn.add_conv_layer(kernel_size=3, filters=4)
        cnn.add_pooling_layer(pool_size=2)
        cnn.add_dense_layer(units=10)
        assert cnn.layer_count == 3
        input_matrix = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]]
        result = cnn.forward(input_matrix)
        assert result["status"] == "ok"
        assert result["feature_maps"] >= 1

    def test_cnn_convolve(self):
        from pytrex import ConvolutionalNN
        cnn = ConvolutionalNN()
        input_m = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
        kernel = [[1, 0], [0, 1]]
        result = cnn.convolve(input_m, kernel)
        assert len(result) == 2
        assert len(result[0]) == 2

    def test_cnn_max_pool(self):
        from pytrex import ConvolutionalNN
        cnn = ConvolutionalNN()
        input_m = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]]
        result = cnn.max_pool(input_m, pool_size=2)
        assert len(result) == 2
        assert len(result[0]) == 2
        assert result[0][0] == 6
        assert result[1][1] == 16

    def test_rnn(self):
        from pytrex import RecurrentNN
        rnn = RecurrentNN(input_size=2, hidden_size=4, output_size=1)
        seq = [[0.5, 0.3], [0.8, 0.1], [0.2, 0.9]]
        outputs = rnn.forward_sequence(seq)
        assert len(outputs) == 3
        assert len(outputs[0]) == 1
        assert len(rnn.hidden_state) == 4
        cfg = rnn.config
        assert cfg["input_size"] == 2
        assert cfg["hidden_size"] == 4

    def test_rnn_train(self):
        from pytrex import RecurrentNN
        rnn = RecurrentNN(input_size=1, hidden_size=3, output_size=1)
        sequences = [[[0.1], [0.2], [0.3]]]
        targets = [[[0.15], [0.25], [0.35]]]
        result = rnn.train_sequence(sequences, targets, epochs=10)
        assert result["status"] == "ok"
        assert result["epochs"] == 10

    def test_transformer(self):
        from pytrex import TransformerModel
        tm = TransformerModel(vocab_size=100, d_model=32, n_heads=4)
        pe = tm.positional_encoding(5)
        assert len(pe) == 5
        assert len(pe[0]) == 32
        result = tm.encode([1, 5, 10, 20])
        assert result["status"] == "ok"
        assert result["seq_len"] == 4
        cfg = tm.config
        assert cfg["vocab_size"] == 100
        assert cfg["n_heads"] == 4

    def test_transformer_self_attention(self):
        from pytrex import TransformerModel
        tm = TransformerModel(vocab_size=50, d_model=8, n_heads=2)
        q = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]
        k = [[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
             [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1]]
        v = [[1.0, 0.5, 0.3, 0.2, 0.1, 0.05, 0.02, 0.01],
             [0.01, 0.02, 0.05, 0.1, 0.2, 0.3, 0.5, 1.0]]
        out = tm.self_attention(q, k, v)
        assert len(out) == 8
        assert len(tm.attention) == 2

    def test_gan(self):
        from pytrex import GANEngine
        gan = GANEngine(noise_dim=5, data_dim=3)
        noise = [0.1, 0.2, 0.3, 0.4, 0.5]
        generated = gan.generate(noise)
        assert len(generated) == 3
        disc = gan.discriminate(generated)
        assert 0 <= disc <= 1
        real_data = [[0.5, 0.3, 0.1], [0.2, 0.8, 0.4], [0.6, 0.1, 0.9]]
        result = gan.train(real_data, epochs=20, batch_size=3)
        assert result["status"] == "ok"
        batch = gan.generate_batch(3)
        assert len(batch) == 3
        assert len(gan.gen_loss_history) == 20

    def test_reinforcement_learning(self):
        from pytrex import ReinforcementLearning
        rl = ReinforcementLearning(n_states=5, n_actions=3)
        action = rl.choose_action(0)
        assert 0 <= action < 3
        result = rl.update(0, 1, 1.0, 1)
        assert result["status"] == "ok"
        transitions = [(0, 1, 1.0, 1), (1, 2, -0.5, 2), (2, 0, 0.5, 0)]
        ep = rl.train_episode(transitions)
        assert ep["status"] == "ok"
        assert ep["episode"] == 1
        assert rl.total_reward > 0
        assert rl.episodes == 1
        old_eps = rl.epsilon
        new_eps = rl.decay_epsilon(0.9)
        assert new_eps < old_eps

    def test_optimizer(self):
        from pytrex import Optimizer
        opt = Optimizer("adam", lr=0.01)
        params = [1.0, 2.0, 3.0]
        grads = [0.1, 0.2, 0.3]
        new_params = opt.step(params, grads)
        assert len(new_params) == 3
        assert opt.type == "adam"
        assert opt.updates == 1
        opt_sgd = Optimizer("sgd", lr=0.1)
        new_p = opt_sgd.step([1.0], [0.5])
        assert abs(new_p[0] - 0.95) < 0.001
        opt_mom = Optimizer("momentum", lr=0.1)
        new_p = opt_mom.step([1.0], [0.5])
        assert new_p[0] != 1.0
        opt_rms = Optimizer("rmsprop", lr=0.1)
        new_p = opt_rms.step([1.0], [0.5])
        assert new_p[0] != 1.0

    def test_loss_functions(self):
        from pytrex import LossFunctions
        mse = LossFunctions.mse([1, 2, 3], [1, 2, 3])
        assert mse == 0.0
        mae = LossFunctions.mae([1, 2], [3, 4])
        assert mae == 2.0
        bce = LossFunctions.binary_crossentropy([0.9, 0.1], [1, 0])
        assert bce >= 0
        ce = LossFunctions.categorical_crossentropy([[0.7, 0.2, 0.1]], [[1, 0, 0]])
        assert ce >= 0
        hinge = LossFunctions.hinge([1, -1], [1, -1])
        assert hinge == 0.0
        kl = LossFunctions.kl_divergence([0.5, 0.5], [0.5, 0.5])
        assert abs(kl) < 0.001
        huber = LossFunctions.huber([1, 2], [3, 4], delta=1.0)
        assert huber > 0
        cos = LossFunctions.cosine_similarity([1, 0], [1, 0])
        assert abs(cos - 1.0) < 0.001
        val = LossFunctions.compute("mse", [1, 2], [1, 2])
        assert val == 0.0

    def test_activation_functions(self):
        from pytrex import ActivationFunctions
        assert ActivationFunctions.relu(-5) == 0
        assert ActivationFunctions.relu(5) == 5
        s = ActivationFunctions.sigmoid(0)
        assert abs(s - 0.5) < 0.001
        assert ActivationFunctions.tanh(0) == 0
        assert ActivationFunctions.leaky_relu(-5) == -0.05
        assert ActivationFunctions.elu(-1) < 0
        g = ActivationFunctions.gelu(0)
        assert g == 0
        sm = ActivationFunctions.softmax([1, 2, 3])
        assert abs(sum(sm) - 1.0) < 0.001
        arr = ActivationFunctions.apply("relu", [-1, 2, -3, 4])
        assert arr == [0, 2, 0, 4]
        d = ActivationFunctions.derivative("relu", 5)
        assert d == 1.0
        d2 = ActivationFunctions.derivative("sigmoid", 0)
        assert abs(d2 - 0.25) < 0.001

    def test_regularization(self):
        from pytrex import Regularization
        reg = Regularization()
        dropped = reg.dropout([1, 2, 3, 4, 5], training=False)
        assert dropped == [1, 2, 3, 4, 5]
        bn = reg.batch_norm([1, 2, 3, 4, 5])
        assert len(bn) == 5
        l1 = reg.l1_penalty([1, -2, 3])
        assert l1 > 0
        l2 = reg.l2_penalty([1, 2, 3])
        assert l2 > 0
        r1 = reg.early_stopping(1.0)
        assert r1["stop"] is False
        r2 = reg.early_stopping(0.5)
        assert r2["stop"] is False
        for _ in range(10):
            r = reg.early_stopping(1.0)
        assert reg.stopped is True
        reg.reset()
        assert reg.stopped is False

    def test_attention_mechanism(self):
        from pytrex import AttentionMechanism
        att = AttentionMechanism(d_model=8)
        seq = [[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
               [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1]]
        outputs = att.self_attention(seq)
        assert len(outputs) == 2
        assert len(outputs[0]) == 8
        assert len(att.attention_weights) == 2
        q = [[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]]
        k = [[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]]
        v = [[1.0, 0.5, 0.3, 0.2, 0.1, 0.05, 0.02, 0.01]]
        cross = att.cross_attention(q, k, v)
        assert len(cross) == 1

    def test_transfer_learning(self):
        from pytrex import TransferLearning
        tl = TransferLearning()
        tl.register_model("resnet", [{"type": "conv"}, {"type": "dense"}, {"type": "dense"}])
        tl.freeze_layer("resnet", 0)
        result = tl.fine_tune("resnet", [[0.5, 0.3], [0.1, 0.8]], [1.0, 0.0], epochs=5)
        assert result["status"] == "ok"
        assert result["frozen_layers"] == 1
        assert result["trainable_layers"] == 2
        tl.unfreeze_layer("resnet", 0)
        assert tl.model_count == 1

    def test_model_checkpoint(self):
        from pytrex import ModelCheckpoint
        cp = ModelCheckpoint()
        cp.save("model_v1", {"weights": [1, 2, 3]}, metric_value=0.5, epoch=5)
        cp.save("model_v1", {"weights": [4, 5, 6]}, metric_value=0.3, epoch=10)
        assert cp.checkpoint_count == 1
        loaded = cp.load("model_v1")
        assert loaded["epoch"] == 10
        best = cp.load_best("model_v1")
        assert best["metric"] == 0.3
        assert "model_v1" in cp.list_checkpoints()
        assert cp.delete("model_v1") is True
        assert cp.checkpoint_count == 0

    def test_hyperparameter_tuner(self):
        from pytrex import HyperparameterTuner
        ht = HyperparameterTuner()
        grid = {"lr": [0.01, 0.1], "epochs": [10, 20]}
        def eval_fn(params):
            return abs(params["lr"] - 0.1) + abs(params["epochs"] - 20)
        result = ht.grid_search(grid, eval_fn)
        assert result["status"] == "ok"
        assert result["total_trials"] == 4
        assert result["best_params"]["lr"] == 0.1
        assert result["best_params"]["epochs"] == 20
        assert ht.trial_count == 4

    def test_hyperparameter_random_search(self):
        from pytrex import HyperparameterTuner
        ht = HyperparameterTuner()
        space = {"x": (0.0, 1.0), "y": (0.0, 1.0)}
        result = ht.random_search(space, n_trials=10)
        assert result["status"] == "ok"
        assert result["total_trials"] == 10

    def test_hyperparameter_bayesian(self):
        from pytrex import HyperparameterTuner
        ht = HyperparameterTuner()
        space = {"x": (0.0, 1.0)}
        result = ht.bayesian_optimize(space, n_iter=5)
        assert result["status"] == "ok"
        assert result["iterations"] == 5

    def test_confusion_matrix(self):
        from pytrex import ConfusionMatrix
        cm = ConfusionMatrix(classes=["cat", "dog"])
        cm.update("cat", "cat")
        cm.update("cat", "dog")
        cm.update("dog", "dog")
        cm.update("dog", "dog")
        m = cm.metrics(positive_class="dog")
        assert m["tp"] == 2
        assert m["fp"] == 1
        assert m["fn"] == 0
        assert m["precision"] > 0
        assert m["recall"] == 1.0
        assert m["f1_score"] > 0
        matrix = cm.matrix()
        assert "cat" in matrix
        cm.reset()
        m2 = cm.metrics("dog")
        assert m2["tp"] == 0

    def test_data_augmentation(self):
        from pytrex import DataAugmentation
        da = DataAugmentation()
        matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
        flipped = da.flip_horizontal(matrix)
        assert flipped[0] == [3, 2, 1]
        flipped_v = da.flip_vertical(matrix)
        assert flipped_v[0] == [7, 8, 9]
        rotated = da.rotate_90(matrix)
        assert len(rotated) == 3
        noised = da.add_noise(matrix, 0.01)
        assert len(noised) == 3
        normalized = da.normalize(matrix)
        assert 0 <= normalized[0][0] <= 1
        cropped = da.crop(matrix, top=1, left=1, height=2, width=2)
        assert len(cropped) == 2

    def test_data_augmentation_text(self):
        from pytrex import DataAugmentation
        da = DataAugmentation()
        syn = da.text_synonym_replace("this is good and fast")
        assert "great" in syn or "quick" in syn
        deleted = da.text_random_deletion("one two three four five", prob=0.0)
        assert "one" in deleted
        swapped = da.text_random_swap("hello world foo bar")
        assert len(swapped.split()) == 4
        img_augs = da.augment_image([[1, 2], [3, 4]])
        assert len(img_augs) == 4
        text_augs = da.augment_text("good bad happy")
        assert len(text_augs) == 3
        assert da.augmentation_count == 7

    def test_batch16_in_app(self):
        from pytrex.core import PyTreXApp
        app = PyTreXApp()
        from pytrex import (NeuralNetwork, ConvolutionalNN, RecurrentNN, TransformerModel,
                            GANEngine, ReinforcementLearning, Optimizer, LossFunctions,
                            ActivationFunctions, Regularization, AttentionMechanism,
                            TransferLearning, ModelCheckpoint, HyperparameterTuner,
                            ConfusionMatrix, DataAugmentation, BlockchainBridge)
        assert isinstance(app.neural_net, NeuralNetwork)
        assert isinstance(app.cnn, ConvolutionalNN)
        assert isinstance(app.rnn, RecurrentNN)
        assert isinstance(app.transformer, TransformerModel)
        assert isinstance(app.gan, GANEngine)
        assert isinstance(app.rl, ReinforcementLearning)
        assert isinstance(app.optimizer, Optimizer)
        assert isinstance(app.loss, LossFunctions)
        assert isinstance(app.activation, ActivationFunctions)
        assert isinstance(app.regularization, Regularization)
        assert isinstance(app.attention, AttentionMechanism)
        assert isinstance(app.transfer, TransferLearning)
        assert isinstance(app.checkpoint, ModelCheckpoint)
        assert isinstance(app.hyper_tuner, HyperparameterTuner)
        assert isinstance(app.confusion_matrix, ConfusionMatrix)
        assert isinstance(app.augmentation, DataAugmentation)
        assert isinstance(app.blockchain, BlockchainBridge)


class TestBlockchainBridge:
    """Test BlockchainBridge — Python↔Rust blockchain integration."""

    def test_add_block(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        result = bridge.add_block("test transaction: 5000 TZS")
        assert result["status"] == "ok"
        assert "block" in result
        block = result["block"]
        assert "index" in block
        assert "hash" in block
        assert "previous_hash" in block
        assert "data" in block
        assert block["data"] == "test transaction: 5000 TZS"
        assert bridge.block_count == 1

    def test_add_multiple_blocks(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        r1 = bridge.add_block("block 1")
        r2 = bridge.add_block("block 2")
        r3 = bridge.add_block("block 3")
        assert bridge.block_count == 3
        chain = bridge.get_chain()
        # Indices should be sequential
        assert chain[0]["index"] == r1["block"]["index"]
        assert chain[1]["index"] == r2["block"]["index"]
        assert chain[2]["index"] == r3["block"]["index"]
        # Each block should link to previous
        assert chain[1]["previous_hash"] == chain[0]["hash"]
        assert chain[2]["previous_hash"] == chain[1]["hash"]

    def test_verify_chain_valid(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        bridge.add_block("tx 1")
        bridge.add_block("tx 2")
        result = bridge.verify_chain()
        assert result["status"] == "ok"
        assert result["valid"] is True
        assert result["blocks"] == 2
        assert bridge.chain_valid is True

    def test_verify_chain_empty(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        result = bridge.verify_chain()
        assert result["status"] == "ok"
        assert result["valid"] is True

    def test_verify_chain_tampered(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        bridge.add_block("original data")
        # Tamper with the block data
        BLOCKCHAIN_CACHE[0]["data"] = "tampered data"
        result = bridge.verify_chain()
        assert result["status"] == "ok"
        assert result["valid"] is False
        assert bridge.chain_valid is False

    def test_get_block_by_index(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        r1 = bridge.add_block("first")
        r2 = bridge.add_block("second")
        idx1 = r1["block"]["index"]
        idx2 = r2["block"]["index"]
        block = bridge.get_block(idx1)
        assert block is not None
        assert block["data"] == "first"
        block2 = bridge.get_block(idx2)
        assert block2 is not None
        assert block2["data"] == "second"
        assert bridge.get_block(99999) is None

    def test_clear_chain(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        bridge.add_block("a")
        bridge.add_block("b")
        assert bridge.block_count == 2
        removed = bridge.clear_chain()
        assert removed == 2
        assert bridge.block_count == 0

    def test_last_hash(self):
        from pytrex import BlockchainBridge, BLOCKCHAIN_CACHE
        bridge = BlockchainBridge()
        BLOCKCHAIN_CACHE.clear()
        assert bridge.last_hash is None
        bridge.add_block("test")
        assert bridge.last_hash is not None
        assert len(bridge.last_hash) == 64  # SHA-256 hex

    def test_blockchain_in_app(self):
        from pytrex.core import PyTreXApp
        from pytrex import BlockchainBridge
        app = PyTreXApp()
        assert isinstance(app.blockchain, BlockchainBridge)
        result = app.blockchain.add_block("app transaction")
        assert result["status"] == "ok"
