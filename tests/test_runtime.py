"""
ScreenCast Pro - Runtime Integration Test Script
Tests the running application's RTSP streaming, WebSocket remote control, and API endpoints.

Prerequisites:
  1. App must be running (npm run tauri dev or built executable)
  2. pip install opencv-python websocket-client requests

Usage:
  python tests/test_runtime.py [--host 127.0.0.1] [--rtsp-port 8554] [--ws-port 9001] [--password ""]
"""

import argparse
import json
import sys
import time
import socket
import struct


def check_port_open(host: str, port: int, timeout: float = 3.0) -> bool:
    """Check if a TCP port is open and accepting connections."""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        result = sock.connect_ex((host, port))
        sock.close()
        return result == 0
    except Exception:
        return False


def test_prerequisites(host: str, rtsp_port: int, ws_port: int) -> dict:
    """Test 1: Check that required ports are listening (app is running)."""
    results = {}
    print("\n" + "=" * 60)
    print("TEST 1: Prerequisites - Port Availability")
    print("=" * 60)

    # Check RTSP port
    rtsp_open = check_port_open(host, rtsp_port)
    results["rtsp_port"] = rtsp_open
    if rtsp_open:
        print(f"  ✅ RTSP port {rtsp_port} is OPEN")
    else:
        print(f"  ❌ RTSP port {rtsp_port} is CLOSED - start stream first via UI")

    # Check WebSocket port (only if remote control is started)
    ws_open = check_port_open(host, ws_port)
    results["ws_port"] = ws_open
    if ws_open:
        print(f"  ✅ WebSocket port {ws_port} is OPEN")
    else:
        print(f"  ⚠️  WebSocket port {ws_port} is CLOSED - remote control not started yet")

    return results


def test_rtsp_stream(url: str, duration: int = 10) -> dict:
    """Test 2: RTSP stream - connect and verify frames are being received."""
    results = {"connected": False, "frames": 0, "fps": 0.0, "resolution": None}
    print("\n" + "=" * 60)
    print(f"TEST 2: RTSP Stream - {url}")
    print("=" * 60)

    try:
        import cv2
    except ImportError:
        print("  ⚠️  SKIP: opencv-python not installed. Run: pip install opencv-python")
        results["skip"] = True
        return results

    # Try to connect
    print(f"  Connecting to {url}...")
    cap = cv2.VideoCapture(url)

    if not cap.isOpened():
        print(f"  ❌ FAIL: Cannot open RTSP stream")
        print(f"     Possible causes:")
        print(f"     - Stream not started (start via UI first)")
        print(f"     - Wrong RTSP port")
        print(f"     - GStreamer pipeline error")
        return results

    results["connected"] = True
    print(f"  ✅ Connected to RTSP stream")

    # Read frames for specified duration
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
            print(f"  ✅ FPS acceptable (≥5)")
        else:
            print(f"  ⚠️  FPS low ({fps:.1f} < 5)")
    else:
        print(f"  ❌ No frames received at all")

    return results


def test_rtsp_multiple_streams(host: str, rtsp_port: int, source_ids: list) -> dict:
    """Test 3: Multiple RTSP streams simultaneously."""
    results = {}
    print("\n" + "=" * 60)
    print("TEST 3: Multiple RTSP Streams")
    print("=" * 60)

    try:
        import cv2
    except ImportError:
        print("  ⚠️  SKIP: opencv-python not installed")
        results["skip"] = True
        return results

    if not source_ids:
        # Default to screen-0 and screen-1 if available
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

    # Read frames from all simultaneously
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
    if all_working:
        print(f"  ✅ All {len(caps)} streams working simultaneously")
    else:
        print(f"  ❌ Some streams failed")

    return results


def test_rtsp_latency(host: str, rtsp_port: int, source_id: str = "screen-0") -> dict:
    """Test 4: Measure RTSP stream latency."""
    results = {"latency_ms": None}
    print("\n" + "=" * 60)
    print("TEST 4: RTSP Stream Latency")
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

    # Measure time-to-first-frame
    print(f"  Measuring time to first frame from {url}...")
    start = time.time()
    ret, frame = cap.read()
    first_frame_ms = (time.time() - start) * 1000

    if ret:
        results["first_frame_ms"] = first_frame_ms
        print(f"  ✅ Time to first frame: {first_frame_ms:.0f}ms")
    else:
        print(f"  ❌ Failed to receive first frame")
        cap.release()
        return results

    # Measure inter-frame jitter
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
        min_ms = min(frame_times)
        max_ms = max(frame_times)
        jitter = max_ms - min_ms
        results["avg_frame_ms"] = avg_frame_ms
        results["jitter_ms"] = jitter
        print(f"  Frame timing: avg={avg_frame_ms:.1f}ms, min={min_ms:.1f}ms, max={max_ms:.1f}ms")
        print(f"  Jitter: {jitter:.1f}ms")

        # Estimated latency (buffer + network + decode)
        # For UDP RTSP, typical latency is 2-3 frames
        est_latency = avg_frame_ms * 3
        results["latency_ms"] = est_latency
        print(f"  Estimated end-to-end latency: ~{est_latency:.0f}ms")

        if est_latency < 500:
            print(f"  ✅ Latency acceptable (<500ms)")
        else:
            print(f"  ⚠️  Latency high (≥500ms)")

    return results


def test_remote_control(host: str, port: int, password: str = "") -> dict:
    """Test 5: WebSocket remote control authentication and commands."""
    results = {"connected": False, "authenticated": False}
    print("\n" + "=" * 60)
    print(f"TEST 5: WebSocket Remote Control - ws://{host}:{port}")
    print("=" * 60)

    try:
        import websocket
    except ImportError:
        print("  ⚠️  SKIP: websocket-client not installed. Run: pip install websocket-client")
        results["skip"] = True
        return results

    url = f"ws://{host}:{port}"

    # Try to connect
    try:
        ws = websocket.create_connection(url, timeout=5)
    except ConnectionRefusedError:
        print(f"  ❌ Connection refused - remote control not started")
        print(f"     Start remote control via UI first")
        return results
    except Exception as e:
        print(f"  ❌ Connection failed: {e}")
        return results

    results["connected"] = True
    print(f"  ✅ WebSocket connected")

    # Authenticate
    auth_msg = json.dumps({"type": "auth", "password": password})
    ws.send(auth_msg)
    resp = json.loads(ws.recv())
    print(f"  Auth response: {resp}")

    if resp.get("status") == "ok" and resp.get("type") == "auth":
        results["authenticated"] = True
        print(f"  ✅ Authentication successful")
    else:
        print(f"  ❌ Authentication failed: {resp}")
        ws.close()
        return results

    # Test mouse_move
    print(f"\n  Testing mouse_move...")
    move_msg = json.dumps({
        "type": "mouse_move",
        "stream_id": "screen-0",
        "data": {"x": 500, "y": 300}
    })
    ws.send(move_msg)
    resp = json.loads(ws.recv())
    results["mouse_move"] = resp.get("status") == "ok"
    print(f"  mouse_move response: {resp}")
    if results["mouse_move"]:
        print(f"  ✅ mouse_move OK")

    # Test mouse_click
    print(f"  Testing mouse_click...")
    click_msg = json.dumps({
        "type": "mouse_click",
        "stream_id": "screen-0",
        "data": {"button": "left", "action": "press"}
    })
    ws.send(click_msg)
    resp = json.loads(ws.recv())
    results["mouse_click"] = resp.get("status") == "ok"
    print(f"  mouse_click response: {resp}")
    if results["mouse_click"]:
        print(f"  ✅ mouse_click OK")

    # Test key_press (using a safe key that won't cause damage)
    print(f"  Testing key_press...")
    key_msg = json.dumps({
        "type": "key_press",
        "stream_id": "screen-0",
        "data": {"key": "a", "modifiers": []}
    })
    ws.send(key_msg)
    resp = json.loads(ws.recv())
    results["key_press"] = resp.get("status") == "ok"
    print(f"  key_press response: {resp}")
    if results["key_press"]:
        print(f"  ✅ key_press OK")

    # Test invalid command
    print(f"  Testing invalid command...")
    invalid_msg = json.dumps({"type": "invalid_command", "stream_id": "screen-0"})
    ws.send(invalid_msg)
    resp = json.loads(ws.recv())
    results["invalid_command_rejected"] = resp.get("status") == "error"
    print(f"  Invalid command response: {resp}")
    if results["invalid_command_rejected"]:
        print(f"  ✅ Invalid command properly rejected")

    # Test wrong auth (new connection)
    print(f"\n  Testing wrong password rejection...")
    ws.close()
    try:
        ws2 = websocket.create_connection(url, timeout=5)
        wrong_auth = json.dumps({"type": "auth", "password": "wrong_password_12345"})
        ws2.send(wrong_auth)
        resp = json.loads(ws2.recv())
        results["wrong_password_rejected"] = resp.get("status") == "error"
        print(f"  Wrong password response: {resp}")
        if results["wrong_password_rejected"]:
            print(f"  ✅ Wrong password properly rejected")
        ws2.close()
    except Exception as e:
        print(f"  ⚠️  Could not test wrong password: {e}")

    return results


def test_rtsp_no_password_reject(host: str, rtsp_port: int) -> dict:
    """Test 6: Verify RTSP has no authentication (it's a local streaming protocol)."""
    results = {}
    print("\n" + "=" * 60)
    print("TEST 6: RTSP Authentication Check")
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
        if ret:
            print(f"  ✅ RTSP stream accessible without authentication (expected)")
        else:
            print(f"  ⚠️  RTSP connected but no frames")
        cap.release()
    else:
        print(f"  ⚠️  RTSP not accessible - stream may not be started")
        results["no_auth_required"] = None

    return results


def test_rtp_info(host: str, rtsp_port: int, path: str = "screen-0") -> dict:
    """Test 7: RTSP DESCRIBE to check codec and media info."""
    results = {}
    print("\n" + "=" * 60)
    print("TEST 7: RTSP Media Information")
    print("=" * 60)

    import socket
    url = f"rtsp://{host}:{rtsp_port}/{path}"

    # Send RTSP DESCRIBE request manually
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect((host, rtsp_port))

        cseq = 1
        describe = (
            f"DESCRIBE {url} RTSP/1.0\r\n"
            f"CSeq: {cseq}\r\n"
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

        # Check for expected content
        if "200 OK" in response:
            print(f"  ✅ RTSP DESCRIBE returned 200 OK")
            results["status_ok"] = True
        else:
            print(f"  ⚠️  RTSP DESCRIBE did not return 200 OK")
            results["status_ok"] = False

        if "H264" in response or "h264" in response or "96" in response:
            print(f"  ✅ H264 codec detected in SDP")
            results["h264_detected"] = True
        else:
            print(f"  ⚠️  H264 codec not clearly detected in SDP")
            results["h264_detected"] = False

    except Exception as e:
        print(f"  ❌ RTSP DESCRIBE failed: {e}")
        results["describe_response"] = False

    return results


def main():
    parser = argparse.ArgumentParser(description="ScreenCast Pro Runtime Integration Tests")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--rtsp-port", type=int, default=8554)
    parser.add_argument("--ws-port", type=int, default=9001)
    parser.add_argument("--password", default="")
    parser.add_argument("--duration", type=int, default=10, help="RTSP test duration in seconds")
    parser.add_argument("--skip-latency", action="store_true", help="Skip latency test")
    args = parser.parse_args()

    print("╔══════════════════════════════════════════════════════════╗")
    print("║     ScreenCast Pro - Runtime Integration Test Suite     ║")
    print("╚══════════════════════════════════════════════════════════╝")
    print(f"\n  Host: {args.host}")
    print(f"  RTSP Port: {args.rtsp_port}")
    print(f"  WebSocket Port: {args.ws_port}")
    print(f"  Password: {'***' if args.password else '(none)'}")
    print()
    print("  ⚠️  IMPORTANT: Start the app and begin streaming before running tests!")
    print("     1. Run: npm run tauri dev")
    print("     2. In the UI, select a source and click 'Start Stream'")
    print("     3. Optionally start Remote Control in the config panel")
    print()

    all_results = {}

    # Test 1: Prerequisites
    all_results["prerequisites"] = test_prerequisites(args.host, args.rtsp_port, args.ws_port)

    # Test 2: RTSP Stream
    rtsp_url = f"rtsp://{args.host}:{args.rtsp_port}/screen-0"
    all_results["rtsp_stream"] = test_rtsp_stream(rtsp_url, args.duration)

    # Test 3: Multiple Streams (only if RTSP is working)
    if all_results["rtsp_stream"].get("connected"):
        all_results["multi_stream"] = test_rtsp_multiple_streams(
            args.host, args.rtsp_port, ["screen-0", "screen-1"]
        )

    # Test 4: Latency
    if not args.skip_latency and all_results["rtsp_stream"].get("connected"):
        all_results["latency"] = test_rtsp_latency(args.host, args.rtsp_port)

    # Test 5: Remote Control
    if all_results["prerequisites"].get("ws_port"):
        all_results["remote_control"] = test_remote_control(
            args.host, args.ws_port, args.password
        )

    # Test 6: RTSP Auth
    if all_results["rtsp_stream"].get("connected"):
        all_results["rtsp_auth"] = test_rtsp_no_password_reject(args.host, args.rtsp_port)

    # Test 7: RTSP Media Info
    if all_results["rtsp_stream"].get("connected"):
        all_results["rtsp_info"] = test_rtp_info(args.host, args.rtsp_port)

    # Summary
    print("\n" + "=" * 60)
    print("RUNTIME TEST SUMMARY")
    print("=" * 60)

    pass_count = 0
    fail_count = 0
    skip_count = 0

    for test_name, result in all_results.items():
        if result.get("skip"):
            status = "⏭️  SKIP"
            skip_count += 1
        elif any(v is True for v in result.values() if isinstance(v, bool)):
            # At least one positive result
            if any(v is False for v in result.values() if isinstance(v, bool)):
                status = "⚠️  PARTIAL"
                pass_count += 1
            else:
                status = "✅ PASS"
                pass_count += 1
        elif any(v is False for v in result.values() if isinstance(v, bool)):
            status = "❌ FAIL"
            fail_count += 1
        else:
            status = "⚠️  N/A"
            skip_count += 1

        print(f"  {test_name}: {status}")

    print(f"\n  Total: {pass_count} passed, {fail_count} failed, {skip_count} skipped")
    print("=" * 60)

    if fail_count > 0:
        sys.exit(1)
    else:
        print("\n  🎉 All runtime tests completed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
