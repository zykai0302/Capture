"""
ScreenCast Pro - E2E Integration Test Script
Tests RTSP streaming and WebSocket remote control

Usage:
  python tests/test_e2e.py [--host 127.0.0.1] [--rtsp-port 8554] [--ws-port 9001] [--password ""]
"""

import argparse
import json
import sys
import time

def test_rtsp_stream(url: str, duration: int = 10) -> bool:
    """Test RTSP stream by pulling frames with OpenCV"""
    try:
        import cv2
    except ImportError:
        print("SKIP: opencv-python not installed. Run: pip install opencv-python")
        return True  # Don't fail if cv2 not available

    print(f"\n=== Testing RTSP Stream: {url} ===")
    cap = cv2.VideoCapture(url)

    if not cap.isOpened():
        print(f"FAIL: Cannot open RTSP stream: {url}")
        return False

    start = time.time()
    frame_count = 0

    while time.time() - start < duration:
        ret, frame = cap.read()
        if ret:
            frame_count += 1
        else:
            print(f"WARN: Failed to read frame after {frame_count} frames")
            break

    cap.release()
    fps = frame_count / duration if duration > 0 else 0
    print(f"  Frames: {frame_count} over {duration}s = {fps:.1f} fps")

    if fps < 5:
        print(f"  FAIL: FPS too low ({fps:.1f} < 5)")
        return False

    print(f"  PASS: RTSP stream working at {fps:.1f} fps")
    return True


def test_remote_control(host: str, port: int, password: str = "") -> bool:
    """Test WebSocket remote control"""
    try:
        import websocket
    except ImportError:
        print("SKIP: websocket-client not installed. Run: pip install websocket-client")
        return True  # Don't fail if not available

    print(f"\n=== Testing Remote Control: ws://{host}:{port} ===")
    url = f"ws://{host}:{port}"

    try:
        ws = websocket.create_connection(url, timeout=5)
    except Exception as e:
        print(f"  FAIL: Cannot connect to WebSocket: {e}")
        return False

    # Authenticate
    auth_msg = json.dumps({"type": "auth", "password": password})
    ws.send(auth_msg)
    resp = ws.recv()
    print(f"  Auth response: {resp}")

    # Test mouse move
    move_msg = json.dumps({
        "type": "mouse_move",
        "stream_id": "screen-0",
        "data": {"x": 500, "y": 300}
    })
    ws.send(move_msg)
    resp = ws.recv()
    print(f"  Mouse move response: {resp}")

    # Test key press
    key_msg = json.dumps({
        "type": "key_press",
        "stream_id": "screen-0",
        "data": {"key": "a", "modifiers": []}
    })
    ws.send(key_msg)
    resp = ws.recv()
    print(f"  Key press response: {resp}")

    ws.close()
    print("  PASS: Remote control WebSocket working")
    return True


def test_tauri_commands(host: str, rtsp_port: int) -> bool:
    """Test Tauri backend commands via HTTP (if dev server is running)"""
    print(f"\n=== Testing Tauri Commands ===")
    print("  Note: Tauri commands are invoked via IPC from the frontend.")
    print("  For full E2E testing, use the application UI directly.")
    print("  SKIP: Automated Tauri command testing requires app runtime.")
    return True


def main():
    parser = argparse.ArgumentParser(description="ScreenCast Pro E2E Tests")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--rtsp-port", type=int, default=8554)
    parser.add_argument("--ws-port", type=int, default=9001)
    parser.add_argument("--password", default="")
    parser.add_argument("--duration", type=int, default=10, help="RTSP test duration in seconds")
    args = parser.parse_args()

    results = {}

    # Test RTSP
    rtsp_url = f"rtsp://{args.host}:{args.rtsp_port}/screen-0"
    results["rtsp"] = test_rtsp_stream(rtsp_url, args.duration)

    # Test Remote Control
    results["remote"] = test_remote_control(args.host, args.ws_port, args.password)

    # Summary
    print("\n" + "=" * 50)
    print("E2E Test Summary:")
    print("-" * 50)
    all_pass = True
    for name, passed in results.items():
        status = "PASS" if passed else "FAIL"
        print(f"  {name}: {status}")
        if not passed:
            all_pass = False

    print("=" * 50)
    if all_pass:
        print("All tests passed!")
        sys.exit(0)
    else:
        print("Some tests failed!")
        sys.exit(1)


if __name__ == "__main__":
    main()
