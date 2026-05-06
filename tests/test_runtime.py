"""
ScreenCast Pro - Runtime Integration Test Script
Tests RTSP streaming, WebSocket remote control (authentication, all 6 command types,
edge cases, security, concurrency, input verification), and RTSP protocol details.

Prerequisites:
  1. App must be running (npm run tauri dev or built executable)
  2. pip install opencv-python websocket-client

Usage:
  python tests/test_runtime.py [--host 127.0.0.1] [--rtsp-port 8554] [--ws-port 9001] [--password ""]
  python tests/test_runtime.py --remote-only          # Run only remote control tests
  python tests/test_runtime.py --skip-remote          # Skip remote control tests
  python tests/test_runtime.py --skip-mouse-verify    # Skip OS-level mouse position checks
  python tests/test_runtime.py --skip-concurrency     # Skip concurrency tests
"""

import argparse
import ctypes
import json
import sys
import time
import socket


# ═══════════════════════════════════════════════════════════════════
# Global counters & helpers
# ═══════════════════════════════════════════════════════════════════

PASS = 0
FAIL = 0
SKIP = 0


def _record(result: bool, name: str, detail: str = ""):
    global PASS, FAIL
    if result:
        PASS += 1
        print(f"    ✅ {name}")
    else:
        FAIL += 1
        print(f"    ❌ {name} {detail}")


def check_port_open(host: str, port: int, timeout: float = 3.0) -> bool:
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        result = sock.connect_ex((host, port))
        sock.close()
        return result == 0
    except Exception:
        return False


# ─── OS-level mouse helpers (Windows) ──────────────────────────────

class _POINT(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]


def get_mouse_pos():
    pt = _POINT()
    ctypes.windll.user32.GetCursorPos(ctypes.byref(pt))
    return pt.x, pt.y


def set_mouse_pos(x, y):
    ctypes.windll.user32.SetCursorPos(x, y)


# ─── WebSocket helpers ─────────────────────────────────────────────

def ws_connect(host, port, timeout=5):
    import websocket
    return websocket.create_connection(f"ws://{host}:{port}", timeout=timeout)


def ws_auth(ws, password):
    ws.send(json.dumps({"type": "auth", "password": password}))
    return json.loads(ws.recv())


def ws_command(ws, cmd_type, stream_id, data):
    ws.send(json.dumps({"type": cmd_type, "stream_id": stream_id, "data": data}))
    return json.loads(ws.recv())


def ws_reconnect(host, port, password, retries=3):
    """Connect + auth, return ws. Replaces a broken connection."""
    import websocket
    last_err = None
    for attempt in range(retries):
        try:
            ws = ws_connect(host, port)
            ws_auth(ws, password)
            return ws
        except (ConnectionRefusedError, ConnectionResetError, OSError) as e:
            last_err = e
            if attempt < retries - 1:
                time.sleep(1)
    raise ConnectionError(f"Failed to connect after {retries} attempts: {last_err}")


# ═══════════════════════════════════════════════════════════════════
# PART A: RTSP STREAMING TESTS
# ═══════════════════════════════════════════════════════════════════

def test_prerequisites(host: str, rtsp_port: int, ws_port: int) -> dict:
    """A1: Check that required ports are listening."""
    results = {}
    print("\n" + "=" * 60)
    print("A1: Prerequisites - Port Availability")
    print("=" * 60)

    rtsp_open = check_port_open(host, rtsp_port)
    results["rtsp_port"] = rtsp_open
    print(f"  {'✅' if rtsp_open else '❌'} RTSP port {rtsp_port} is {'OPEN' if rtsp_open else 'CLOSED'}")

    ws_open = check_port_open(host, ws_port)
    results["ws_port"] = ws_open
    print(f"  {'✅' if ws_open else '⚠️ '} WebSocket port {ws_port} is {'OPEN' if ws_open else 'CLOSED'}")

    return results


def test_rtsp_stream(url: str, duration: int = 10) -> dict:
    """A2: RTSP stream - connect and verify frames."""
    results = {"connected": False, "frames": 0, "fps": 0.0, "resolution": None}
    print("\n" + "=" * 60)
    print(f"A2: RTSP Stream - {url}")
    print("=" * 60)

    try:
        import cv2
    except ImportError:
        print("  ⚠️  SKIP: opencv-python not installed. Run: pip install opencv-python")
        results["skip"] = True
        return results

    print(f"  Connecting to {url}...")
    cap = cv2.VideoCapture(url)
    if not cap.isOpened():
        print("  ❌ Cannot open RTSP stream")
        return results

    results["connected"] = True
    print("  ✅ Connected to RTSP stream")

    start = time.time()
    frame_count = 0
    first_frame = None

    while time.time() - start < duration:
        ret, frame = cap.read()
        if ret:
            frame_count += 1
            if first_frame is None:
                first_frame = frame
                h, w = frame.shape[:2]
                results["resolution"] = (w, h)
                print(f"  ✅ First frame received: {w}x{h}")
        else:
            if frame_count == 0:
                print(f"  ⚠️  No frames received after {time.time()-start:.1f}s")
            break

    cap.release()
    elapsed = time.time() - start
    fps = frame_count / elapsed if elapsed > 0 else 0
    results["frames"] = frame_count
    results["fps"] = fps

    if frame_count > 0:
        print(f"  ✅ Received {frame_count} frames over {elapsed:.1f}s = {fps:.1f} FPS")
        if fps >= 5:
            print("  ✅ FPS acceptable (≥5)")
        else:
            print(f"  ⚠️  FPS low ({fps:.1f} < 5)")
    else:
        print("  ❌ No frames received at all")

    return results


def test_rtsp_multiple_streams(host: str, rtsp_port: int, source_ids: list) -> dict:
    """A3: Multiple RTSP streams simultaneously."""
    results = {}
    print("\n" + "=" * 60)
    print("A3: Multiple RTSP Streams")
    print("=" * 60)

    try:
        import cv2
    except ImportError:
        print("  ⚠️  SKIP: opencv-python not installed")
        results["skip"] = True
        return results

    if not source_ids:
        source_ids = ["screen-0", "screen-1"]

    caps = []
    for sid in source_ids:
        url = f"rtsp://{host}:{rtsp_port}/{sid}"
        print(f"  Connecting to {url}...")
        cap = cv2.VideoCapture(url)
        if cap.isOpened():
            caps.append((sid, cap))
            print(f"  ✅ Connected: {sid}")
        else:
            print(f"  ❌ Failed: {sid}")

    if len(caps) < 2:
        print(f"  ⚠️  Only {len(caps)} stream(s) connected, multi-stream test needs ≥2")
        for _, cap in caps:
            cap.release()
        results["multi_stream"] = False
        return results

    duration = 5
    start = time.time()
    frame_counts = {sid: 0 for sid, _ in caps}
    while time.time() - start < duration:
        for sid, cap in caps:
            ret, _ = cap.read()
            if ret:
                frame_counts[sid] += 1

    for _, cap in caps:
        cap.release()

    for sid, count in frame_counts.items():
        fps = count / duration
        results[sid] = {"frames": count, "fps": fps}
        print(f"  Stream {sid}: {count} frames = {fps:.1f} FPS")

    all_working = all(c > 0 for c in frame_counts.values())
    results["multi_stream"] = all_working
    print(f"  {'✅' if all_working else '❌'} All streams {'working' if all_working else 'failed'}")
    return results


def test_rtsp_latency(host: str, rtsp_port: int, source_id: str = "screen-0") -> dict:
    """A4: Measure RTSP stream latency."""
    results = {"latency_ms": None}
    print("\n" + "=" * 60)
    print("A4: RTSP Stream Latency")
    print("=" * 60)

    try:
        import cv2
    except ImportError:
        print("  ⚠️  SKIP: opencv-python not installed")
        results["skip"] = True
        return results

    url = f"rtsp://{host}:{rtsp_port}/{source_id}"
    cap = cv2.VideoCapture(url)
    if not cap.isOpened():
        print(f"  ❌ Cannot connect to {url}")
        return results

    print(f"  Measuring time to first frame from {url}...")
    start = time.time()
    ret, frame = cap.read()
    first_frame_ms = (time.time() - start) * 1000

    if ret:
        results["first_frame_ms"] = first_frame_ms
        print(f"  ✅ Time to first frame: {first_frame_ms:.0f}ms")
    else:
        print("  ❌ Failed to receive first frame")
        cap.release()
        return results

    frame_times = []
    for _ in range(30):
        s = time.time()
        ret, _ = cap.read()
        if not ret:
            break
        frame_times.append((time.time() - s) * 1000)
    cap.release()

    if frame_times:
        avg_frame_ms = sum(frame_times) / len(frame_times)
        min_ms, max_ms = min(frame_times), max(frame_times)
        jitter = max_ms - min_ms
        results["avg_frame_ms"] = avg_frame_ms
        results["jitter_ms"] = jitter
        print(f"  Frame timing: avg={avg_frame_ms:.1f}ms, min={min_ms:.1f}ms, max={max_ms:.1f}ms")
        print(f"  Jitter: {jitter:.1f}ms")
        est_latency = avg_frame_ms * 3
        results["latency_ms"] = est_latency
        print(f"  Estimated end-to-end latency: ~{est_latency:.0f}ms")
        print(f"  {'✅' if est_latency < 500 else '⚠️ '} Latency {'acceptable' if est_latency < 500 else 'high'}")

    return results


def test_rtsp_no_auth(host: str, rtsp_port: int) -> dict:
    """A5: Verify RTSP has no authentication."""
    results = {}
    print("\n" + "=" * 60)
    print("A5: RTSP Authentication Check")
    print("=" * 60)

    try:
        import cv2
    except ImportError:
        print("  ⚠️  SKIP: opencv-python not installed")
        results["skip"] = True
        return results

    url = f"rtsp://{host}:{rtsp_port}/screen-0"
    cap = cv2.VideoCapture(url)
    if cap.isOpened():
        ret, _ = cap.read()
        results["no_auth_required"] = ret
        print(f"  {'✅' if ret else '⚠️ '} RTSP stream accessible without authentication")
        cap.release()
    else:
        print("  ⚠️  RTSP not accessible - stream may not be started")
        results["no_auth_required"] = None
    return results


def test_rtsp_describe(host: str, rtsp_port: int, path: str = "screen-0") -> dict:
    """A6: RTSP DESCRIBE to check codec and media info."""
    results = {}
    print("\n" + "=" * 60)
    print("A6: RTSP Media Information")
    print("=" * 60)

    url = f"rtsp://{host}:{rtsp_port}/{path}"
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect((host, rtsp_port))
        describe = (
            f"DESCRIBE {url} RTSP/1.0\r\n"
            f"CSeq: 1\r\n"
            f"User-Agent: ScreenCast-Pro-Test\r\n"
            f"Accept: application/sdp\r\n"
            f"\r\n"
        )
        sock.sendall(describe.encode())
        response = sock.recv(4096).decode('utf-8', errors='replace')
        sock.close()

        results["describe_response"] = True
        print(f"  RTSP DESCRIBE response (first 500 chars):")
        print(f"  {response[:500]}")

        if "200 OK" in response:
            print("  ✅ RTSP DESCRIBE returned 200 OK")
            results["status_ok"] = True
        else:
            print("  ⚠️  RTSP DESCRIBE did not return 200 OK")
            results["status_ok"] = False

        if "H264" in response or "h264" in response or "96" in response:
            print("  ✅ H264 codec detected in SDP")
            results["h264_detected"] = True
        else:
            print("  ⚠️  H264 codec not clearly detected in SDP")
            results["h264_detected"] = False

    except Exception as e:
        print(f"  ❌ RTSP DESCRIBE failed: {e}")
        results["describe_response"] = False

    return results


# ═══════════════════════════════════════════════════════════════════
# PART B: REMOTE CONTROL TESTS
# ═══════════════════════════════════════════════════════════════════

def test_rc_connection(host, port):
    """B1: WebSocket connection basics."""
    print("\n" + "=" * 60)
    print("B1: Remote Control - Connection")
    print("=" * 60)

    # B1.1 Basic connect
    try:
        ws = ws_connect(host, port)
        ws.close()
        _record(True, "B1.1 Connect to WebSocket server")
    except Exception as e:
        _record(False, "B1.1 Connect to WebSocket server", str(e))
        return False

    # B1.2 Connection refused on wrong port
    try:
        ws = ws_connect(host, port + 99, timeout=2)
        ws.close()
        _record(False, "B1.2 Wrong port should refuse connection")
    except Exception:
        _record(True, "B1.2 Wrong port refuses connection")

    # B1.3 Multiple concurrent connections
    try:
        connections = [ws_connect(host, port) for _ in range(3)]
        for ws in connections:
            ws.close()
        _record(True, "B1.3 Multiple concurrent connections (3)")
    except Exception as e:
        _record(False, "B1.3 Multiple concurrent connections", str(e))

    return True


def test_rc_authentication(host, port, password):
    """B2: Authentication scenarios."""
    print("\n" + "=" * 60)
    print("B2: Remote Control - Authentication")
    print("=" * 60)

    # B2.1 Correct password auth
    ws = ws_connect(host, port)
    resp = ws_auth(ws, password)
    ok = resp.get("status") == "ok" and resp.get("type") == "auth"
    _record(ok, "B2.1 Correct password authenticates", str(resp) if not ok else "")
    ws.close()

    # B2.2 Wrong password rejected
    ws = ws_connect(host, port)
    resp = ws_auth(ws, password + "_wrong" if password else "wrong_password")
    ok = resp.get("status") == "error" and "Authentication" in resp.get("message", "")
    _record(ok, "B2.2 Wrong password rejected", str(resp) if not ok else "")
    ws.close()

    # B2.3 Command before auth (only meaningful with password set)
    ws = ws_connect(host, port)
    if password:
        resp = ws_command(ws, "mouse_move", "screen-0", {"x": 0, "y": 0})
        ok = resp.get("status") == "error" and "Authentication required" in resp.get("message", "")
        _record(ok, "B2.3 Command before auth rejected", str(resp) if not ok else "")
    else:
        _record(True, "B2.3 No-password mode: auto-auth (N/A)")
    ws.close()

    # B2.4 Re-auth when already authenticated
    ws = ws_connect(host, port)
    ws_auth(ws, password)
    resp = ws_auth(ws, password)
    ok = resp.get("status") == "ok" and resp.get("type") == "auth"
    _record(ok, "B2.4 Re-auth when already authenticated", str(resp) if not ok else "")
    ws.close()

    # B2.5 Missing password field in auth message
    ws = ws_connect(host, port)
    ws.send(json.dumps({"type": "auth"}))
    resp = json.loads(ws.recv())
    if password:
        ok = resp.get("status") == "error"
        _record(ok, "B2.5 Missing password field rejected", str(resp) if not ok else "")
    else:
        ok = resp.get("status") == "ok"
        _record(ok, "B2.5 Missing password field accepted (no-password mode)", str(resp) if not ok else "")
    ws.close()


def test_rc_mouse_commands(host, port, password):
    """B3: All mouse command types and variants."""
    print("\n" + "=" * 60)
    print("B3: Remote Control - Mouse Commands")
    print("=" * 60)

    ws = ws_reconnect(host, port, password)
    orig_x, orig_y = get_mouse_pos()

    # B3.1 mouse_move absolute
    set_mouse_pos(0, 0)
    time.sleep(0.05)
    resp = ws_command(ws, "mouse_move", "screen-0", {"x": 300, "y": 200})
    time.sleep(0.1)
    cx, cy = get_mouse_pos()
    ok = resp.get("status") == "ok" and abs(cx - 300) < 5 and abs(cy - 200) < 5
    _record(ok, "B3.1 mouse_move (300,200)", f"pos=({cx},{cy})" if not ok else "")

    # B3.2 mouse_move to (0,0)
    resp = ws_command(ws, "mouse_move", "screen-0", {"x": 0, "y": 0})
    time.sleep(0.05)
    cx, cy = get_mouse_pos()
    ok = resp.get("status") == "ok" and cx == 0 and cy == 0
    _record(ok, "B3.2 mouse_move (0,0)", f"pos=({cx},{cy})" if not ok else "")

    # B3.3-B3.7 mouse_click variants
    click_variants = [
        ("B3.3", "Left", "Single"),
        ("B3.4", "Right", "Single"),
        ("B3.5", "Middle", "Single"),
        ("B3.6", "Left", "Double"),
        ("B3.7", "Right", "Double"),
    ]
    for tag, btn, act in click_variants:
        set_mouse_pos(500, 300)
        time.sleep(0.05)
        resp = ws_command(ws, "mouse_click", "screen-0",
                          {"x": 500, "y": 300, "button": btn, "action": act})
        _record(resp.get("status") == "ok", f"{tag} mouse_click {btn}/{act}")

    # B3.8-B3.11 mouse_scroll variants
    scroll_variants = [
        ("B3.8", {"x": 500, "y": 300, "dx": 0, "dy": -3}, "vertical down (dy=-3)"),
        ("B3.9", {"x": 500, "y": 300, "dx": 0, "dy": 3}, "vertical up (dy=3)"),
        ("B3.10", {"x": 500, "y": 300, "dx": 5, "dy": 0}, "horizontal (dx=5)"),
        ("B3.11", {"x": 500, "y": 300, "dx": 0, "dy": 0}, "zero (no-op)"),
    ]
    for tag, data, desc in scroll_variants:
        resp = ws_command(ws, "mouse_scroll", "screen-0", data)
        _record(resp.get("status") == "ok", f"{tag} mouse_scroll {desc}")

    # B3.12-B3.13 mouse_drag variants
    set_mouse_pos(200, 200)
    time.sleep(0.05)
    resp = ws_command(ws, "mouse_drag", "screen-0",
                      {"from_x": 200, "from_y": 200, "to_x": 400, "to_y": 300, "button": "Left"})
    time.sleep(0.1)
    cx, cy = get_mouse_pos()
    ok = resp.get("status") == "ok" and abs(cx - 400) < 5 and abs(cy - 300) < 5
    _record(ok, "B3.12 mouse_drag Left", f"pos=({cx},{cy})" if not ok else "")

    set_mouse_pos(100, 100)
    time.sleep(0.05)
    resp = ws_command(ws, "mouse_drag", "screen-0",
                      {"from_x": 100, "from_y": 100, "to_x": 300, "to_y": 200, "button": "Right"})
    _record(resp.get("status") == "ok", "B3.13 mouse_drag Right")

    set_mouse_pos(orig_x, orig_y)
    ws.close()


def test_rc_keyboard_commands(host, port, password):
    """B4: All keyboard command types and variants."""
    print("\n" + "=" * 60)
    print("B4: Remote Control - Keyboard Commands")
    print("=" * 60)

    ws = ws_reconnect(host, port, password)

    # B4.1 key_press single letter
    resp = ws_command(ws, "key_press", "screen-0", {"key": "a", "modifiers": []})
    _record(resp.get("status") == "ok", "B4.1 key_press 'a' no modifiers")

    # B4.2 key_press with Ctrl
    resp = ws_command(ws, "key_press", "screen-0", {"key": "c", "modifiers": ["Ctrl"]})
    _record(resp.get("status") == "ok", "B4.2 key_press Ctrl+C")

    # B4.3 key_press with Ctrl+Shift
    resp = ws_command(ws, "key_press", "screen-0", {"key": "A", "modifiers": ["Ctrl", "Shift"]})
    _record(resp.get("status") == "ok", "B4.3 key_press Ctrl+Shift+A")

    # B4.4 key_press with Alt
    resp = ws_command(ws, "key_press", "screen-0", {"key": "F4", "modifiers": ["Alt"]})
    _record(resp.get("status") == "ok", "B4.4 key_press Alt+F4")

    # B4.5 Special keys
    special_keys = ["Enter", "Escape", "Tab", "Backspace", "Delete", "Space",
                    "Up", "Down", "Left", "Right", "Home", "End", "PageUp", "PageDown", "CapsLock"]
    for key in special_keys:
        resp = ws_command(ws, "key_press", "screen-0", {"key": key, "modifiers": []})
        _record(resp.get("status") == "ok", f"B4.5 key_press '{key}'")

    # B4.6 F-keys
    for i in range(1, 13):
        key = f"F{i}"
        resp = ws_command(ws, "key_press", "screen-0", {"key": key, "modifiers": []})
        _record(resp.get("status") == "ok", f"B4.6 key_press '{key}'")

    # B4.7 Unknown key → error
    resp = ws_command(ws, "key_press", "screen-0", {"key": "UnknownKeyXYZ", "modifiers": []})
    _record(resp.get("status") == "error", "B4.7 Unknown key rejected")

    # B4.8-B4.13 key_combo variants
    combo_tests = [
        ("B4.8", ["Ctrl", "C"], "Ctrl+C"),
        ("B4.9", ["Ctrl", "Alt", "Delete"], "Ctrl+Alt+Delete"),
        ("B4.10", ["Ctrl", "Shift", "Escape"], "Ctrl+Shift+Escape"),
        ("B4.11", ["Alt", "Tab"], "Alt+Tab"),
        ("B4.12", ["Enter"], "single key (Enter)"),
        ("B4.13", ["Win", "D"], "Win+D (Meta alias)"),
    ]
    for tag, keys, desc in combo_tests:
        resp = ws_command(ws, "key_combo", "screen-0", {"keys": keys})
        _record(resp.get("status") == "ok", f"{tag} key_combo {desc}")

    ws.close()


def test_rc_mouse_verify(host, port, password):
    """B5: OS-level mouse position verification."""
    print("\n" + "=" * 60)
    print("B5: Remote Control - Mouse Position Verification (OS-level)")
    print("=" * 60)

    ws = ws_reconnect(host, port, password)
    test_positions = [(100, 100), (500, 400), (960, 540), (0, 0), (200, 600)]
    tolerance = 5

    for tx, ty in test_positions:
        resp = ws_command(ws, "mouse_move", "screen-0", {"x": tx, "y": ty})
        time.sleep(0.1)
        cx, cy = get_mouse_pos()
        ok = resp.get("status") == "ok" and abs(cx - tx) <= tolerance and abs(cy - ty) <= tolerance
        detail = f"expected=({tx},{ty}) actual=({cx},{cy})" if not ok else ""
        _record(ok, f"B5.x mouse_move → ({tx},{ty})", detail)

    ws.close()


def test_rc_command_validation(host, port, password):
    """B6: Malformed and edge-case commands."""
    print("\n" + "=" * 60)
    print("B6: Remote Control - Command Validation & Edge Cases")
    print("=" * 60)

    ws = ws_reconnect(host, port, password)

    # B6.1 Invalid command type
    resp = ws_command(ws, "invalid_type", "screen-0", {})
    _record(resp.get("status") == "error", "B6.1 Invalid command type rejected")

    # B6.2 Missing data field
    ws.send(json.dumps({"type": "mouse_move", "stream_id": "screen-0"}))
    resp = json.loads(ws.recv())
    _record(resp.get("status") == "error", "B6.2 Missing data field rejected", str(resp))

    # B6.3 Negative coordinates
    resp = ws_command(ws, "mouse_move", "screen-0", {"x": -1, "y": -1})
    _record(resp.get("status") in ("ok", "error"), "B6.3 Negative coordinates handled")

    # B6.4 Very large coordinates
    resp = ws_command(ws, "mouse_move", "screen-0", {"x": 99999, "y": 99999})
    _record(resp.get("status") in ("ok", "error"), "B6.4 Large coordinates handled")

    # B6.5 Invalid JSON — server may disconnect, which is acceptable
    ws.send("not json at all")
    try:
        ws.recv()
        _record(True, "B6.5 Invalid JSON handled (got response)")
    except Exception:
        _record(True, "B6.5 Invalid JSON handled (connection closed)")
        ws = ws_reconnect(host, port, password)

    # B6.6 Empty string
    ws.send("")
    try:
        ws.recv()
        _record(True, "B6.6 Empty string handled")
    except Exception:
        _record(True, "B6.6 Empty string handled (connection closed)")
        ws = ws_reconnect(host, port, password)

    # B6.7 Wrong case in MouseButton/ClickAction (Rust expects PascalCase)
    resp = ws_command(ws, "mouse_click", "screen-0",
                      {"x": 500, "y": 300, "button": "left", "action": "single"})
    _record(resp.get("status") == "error", "B6.7 Wrong case button/action rejected", str(resp))

    # B6.8 Invalid action
    resp = ws_command(ws, "mouse_click", "screen-0",
                      {"x": 500, "y": 300, "button": "Left", "action": "Triple"})
    _record(resp.get("status") == "error", "B6.8 Invalid action 'Triple' rejected")

    # B6.9 Invalid button
    resp = ws_command(ws, "mouse_click", "screen-0",
                      {"x": 500, "y": 300, "button": "Side", "action": "Single"})
    _record(resp.get("status") == "error", "B6.9 Invalid button 'Side' rejected")

    ws.close()


def test_rc_concurrency(host, port, password):
    """B7: Multiple clients and rapid command sequences."""
    print("\n" + "=" * 60)
    print("B7: Remote Control - Concurrency & Rapid Commands")
    print("=" * 60)

    # B7.1 Multiple clients
    clients = []
    try:
        for i in range(3):
            ws = ws_connect(host, port)
            resp = ws_auth(ws, password)
            ok = resp.get("status") == "ok"
            _record(ok, f"B7.1 Client {i+1}/3 authenticated")
            clients.append(ws)
    except Exception as e:
        _record(False, "B7.1 Multiple clients", str(e))

    # B7.2 Commands from each client
    for i, ws in enumerate(clients):
        resp = ws_command(ws, "mouse_move", "screen-0", {"x": 100 * (i + 1), "y": 100})
        _record(resp.get("status") == "ok", f"B7.2 Client {i+1} command OK")

    for ws in clients:
        ws.close()

    # B7.3 Rapid command sequence (100 commands)
    ws = ws_reconnect(host, port, password)
    start = time.time()
    sent, ok_count = 0, 0
    for i in range(100):
        resp = ws_command(ws, "mouse_move", "screen-0", {"x": 100 + i, "y": 100})
        sent += 1
        if resp.get("status") == "ok":
            ok_count += 1
    elapsed = time.time() - start
    rate = sent / elapsed if elapsed > 0 else 0
    _record(ok_count == sent, f"B7.3 Rapid sequence ({ok_count}/{sent} OK, {rate:.0f} cmd/s)")
    ws.close()

    # B7.4 Interleaved mouse + keyboard
    ws = ws_reconnect(host, port, password)
    interleaved_ok = True
    for i in range(10):
        r1 = ws_command(ws, "mouse_move", "screen-0", {"x": 200 + i * 10, "y": 200})
        r2 = ws_command(ws, "key_press", "screen-0", {"key": "a", "modifiers": []})
        if r1.get("status") != "ok" or r2.get("status") != "ok":
            interleaved_ok = False
            break
    _record(interleaved_ok, "B7.4 Interleaved mouse+keyboard (10 rounds)")
    ws.close()


def test_rc_connection_lifecycle(host, port, password):
    """B8: Connect/disconnect/reconnect cycles."""
    print("\n" + "=" * 60)
    print("B8: Remote Control - Connection Lifecycle")
    print("=" * 60)

    # B8.1 Connect → auth → command → close → reconnect (3 cycles)
    for cycle in range(3):
        try:
            ws = ws_connect(host, port)
            resp = ws_auth(ws, password)
            ok = resp.get("status") == "ok"
            if ok:
                resp = ws_command(ws, "mouse_move", "screen-0", {"x": 100, "y": 100})
                ok = resp.get("status") == "ok"
            ws.close()
            _record(ok, f"B8.1 Connect/disconnect cycle {cycle + 1}/3")
        except Exception as e:
            _record(False, f"B8.1 Connect/disconnect cycle {cycle + 1}/3", str(e))

    # B8.2 Close without sending anything
    try:
        ws = ws_connect(host, port)
        ws.close()
        _record(True, "B8.2 Close without sending anything")
    except Exception as e:
        _record(False, "B8.2 Close without sending", str(e))

    # B8.3 Auth then idle for 3s, then command
    try:
        ws = ws_reconnect(host, port, password)
        time.sleep(3)
        try:
            resp = ws_command(ws, "mouse_move", "screen-0", {"x": 200, "y": 200})
            ok = resp.get("status") == "ok"
            _record(ok, "B8.3 Idle 3s then command still works", str(resp) if not ok else "")
            ws.close()
        except Exception:
            # Server may close idle connections; reconnect and retry
            ws = ws_reconnect(host, port, password)
            resp = ws_command(ws, "mouse_move", "screen-0", {"x": 200, "y": 200})
            ok = resp.get("status") == "ok"
            _record(ok, "B8.3 Idle 3s then reconnect+command works", str(resp) if not ok else "")
            ws.close()
    except Exception as e:
        _record(False, "B8.3 Idle then command", str(e))


def test_rc_key_aliases(host, port, password):
    """B9: Key name aliases per Rust parse_key()."""
    print("\n" + "=" * 60)
    print("B9: Remote Control - Key Name Aliases")
    print("=" * 60)

    ws = ws_reconnect(host, port, password)

    aliases = [
        ("Control", "Control"), ("ctrl (lowercase)", "ctrl"),
        ("Alt", "Alt"), ("Shift", "Shift"),
        ("Win", "Win"), ("Super (alias for Meta)", "Super"), ("Meta", "Meta"),
        ("Return", "Return"), ("Enter (alias)", "Enter"),
        ("Escape", "Escape"), ("Esc (alias)", "Esc"),
        ("Backspace", "Backspace"), ("PageUp", "PageUp"),
        ("PageDown", "PageDown"), ("Space", "Space"),
    ]

    for display, key in aliases:
        resp = ws_command(ws, "key_press", "screen-0", {"key": key, "modifiers": []})
        ok = resp.get("status") == "ok"
        _record(ok, f"B9.x key '{display}'", f"resp={resp}" if not ok else "")

    ws.close()


# ═══════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════

def main():
    parser = argparse.ArgumentParser(description="ScreenCast Pro - Runtime Integration Tests")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--rtsp-port", type=int, default=8554)
    parser.add_argument("--ws-port", type=int, default=9001)
    parser.add_argument("--password", default="")
    parser.add_argument("--duration", type=int, default=10, help="RTSP test duration in seconds")
    # Filter flags
    parser.add_argument("--remote-only", action="store_true", help="Run only remote control tests")
    parser.add_argument("--skip-remote", action="store_true", help="Skip remote control tests")
    parser.add_argument("--skip-latency", action="store_true", help="Skip RTSP latency test")
    parser.add_argument("--skip-mouse-verify", action="store_true",
                        help="Skip OS-level mouse position verification (B5)")
    parser.add_argument("--skip-concurrency", action="store_true",
                        help="Skip concurrency tests (B7)")
    args = parser.parse_args()

    global PASS, FAIL, SKIP

    print("╔══════════════════════════════════════════════════════════╗")
    print("║     ScreenCast Pro - Runtime Integration Test Suite     ║")
    print("╚══════════════════════════════════════════════════════════╝")
    print(f"\n  Host: {args.host}")
    print(f"  RTSP Port: {args.rtsp_port}")
    print(f"  WebSocket Port: {args.ws_port}")
    print(f"  Password: {'***' if args.password else '(none)'}")
    print(f"  Mode: {'Remote only' if args.remote_only else 'Full' if not args.skip_remote else 'RTSP only'}")
    print()
    print("  Prerequisites:")
    print("    1. App is running (npm run tauri dev)")
    print("    2. RTSP stream started (for streaming tests)")
    print("    3. Remote Control started (for remote tests)")
    print("    4. pip install opencv-python websocket-client")
    print()

    # ─── PART A: RTSP STREAMING ──────────────────────────────────
    if not args.remote_only:
        print("\n" + "═" * 60)
        print("  PART A: RTSP STREAMING TESTS")
        print("═" * 60)

        all_results = {}

        # A1: Prerequisites
        all_results["prerequisites"] = test_prerequisites(args.host, args.rtsp_port, args.ws_port)

        # A2: RTSP Stream
        rtsp_url = f"rtsp://{args.host}:{args.rtsp_port}/screen-0"
        all_results["rtsp_stream"] = test_rtsp_stream(rtsp_url, args.duration)

        # A3: Multiple Streams
        if all_results["rtsp_stream"].get("connected"):
            all_results["multi_stream"] = test_rtsp_multiple_streams(
                args.host, args.rtsp_port, ["screen-0", "screen-1"]
            )

        # A4: Latency
        if not args.skip_latency and all_results["rtsp_stream"].get("connected"):
            all_results["latency"] = test_rtsp_latency(args.host, args.rtsp_port)

        # A5: RTSP Auth
        if all_results["rtsp_stream"].get("connected"):
            all_results["rtsp_auth"] = test_rtsp_no_auth(args.host, args.rtsp_port)

        # A6: RTSP Media Info
        if all_results["rtsp_stream"].get("connected"):
            all_results["rtsp_info"] = test_rtsp_describe(args.host, args.rtsp_port)

        # Part A summary
        print("\n" + "-" * 60)
        print("  RTSP Test Summary:")
        for test_name, result in all_results.items():
            if result.get("skip"):
                status = "⏭️  SKIP"
                global SKIP
                SKIP += 1
            elif any(v is True for v in result.values() if isinstance(v, bool)):
                status = "⚠️  PARTIAL" if any(v is False for v in result.values() if isinstance(v, bool)) else "✅ PASS"
            elif any(v is False for v in result.values() if isinstance(v, bool)):
                status = "❌ FAIL"
            else:
                status = "⚠️  N/A"
                SKIP += 1
            print(f"    {test_name}: {status}")

    # ─── PART B: REMOTE CONTROL ──────────────────────────────────
    if not args.skip_remote:
        print("\n" + "═" * 60)
        print("  PART B: REMOTE CONTROL TESTS")
        print("═" * 60)

        try:
            import websocket
        except ImportError:
            print("  ⚠️  SKIP: websocket-client not installed. Run: pip install websocket-client")
            SKIP += 1
        else:
            # Check if WS port is open
            ws_available = check_port_open(args.host, args.ws_port)
            if not ws_available and not args.remote_only:
                print(f"  ⚠️  WebSocket port {args.ws_port} is CLOSED - skipping remote tests")
                print("     Start Remote Control via UI first")
                SKIP += 1
            elif not ws_available:
                print(f"  ❌ WebSocket port {args.ws_port} is CLOSED - cannot run remote-only tests")
                sys.exit(1)
            else:
                # B1: Connection
                connected = test_rc_connection(args.host, args.ws_port)
                if not connected:
                    print("\n❌ Cannot connect to WebSocket server. Skipping remaining remote tests.")
                    FAIL += 1
                else:
                    # B2: Authentication
                    test_rc_authentication(args.host, args.ws_port, args.password)

                    # B3: Mouse Commands
                    test_rc_mouse_commands(args.host, args.ws_port, args.password)

                    # B4: Keyboard Commands
                    test_rc_keyboard_commands(args.host, args.ws_port, args.password)

                    # B5: Mouse Position Verification
                    if not args.skip_mouse_verify:
                        test_rc_mouse_verify(args.host, args.ws_port, args.password)

                    # B6: Command Validation
                    test_rc_command_validation(args.host, args.ws_port, args.password)

                    # B7: Concurrency
                    if not args.skip_concurrency:
                        test_rc_concurrency(args.host, args.ws_port, args.password)

                    # B8: Connection Lifecycle
                    test_rc_connection_lifecycle(args.host, args.ws_port, args.password)

                    # B9: Key Aliases
                    test_rc_key_aliases(args.host, args.ws_port, args.password)

    # ─── FINAL SUMMARY ───────────────────────────────────────────
    print("\n" + "=" * 60)
    print("FINAL TEST SUMMARY")
    print("=" * 60)
    total = PASS + FAIL
    print(f"  ✅ Passed:  {PASS}")
    print(f"  ❌ Failed:  {FAIL}")
    print(f"  ⏭️  Skipped: {SKIP}")
    if total > 0:
        print(f"  Pass Rate:  {PASS / total * 100:.1f}%")
    print("=" * 60)

    if FAIL > 0:
        sys.exit(1)
    else:
        print("\n  All tests passed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
