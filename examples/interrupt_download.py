#!/usr/bin/env python3
"""Authorized fault injection: drop one QA DAT stream, never save its payload.

Usage: python3 examples/interrupt_download.py SETTINGS_JSON DIAGNOSTIC_BINARY
Credentials are entered by the Rust diagnostic's hidden terminal prompts.
"""
import http.client
import http.server
import json
import pathlib
import ssl
import subprocess
import sys
import tempfile
import threading
import urllib.parse


def main():
    settings = json.loads(pathlib.Path(sys.argv[1]).read_text())
    source = urllib.parse.urlsplit(settings.get("dat_url", settings.get("url", "")))
    if (source.scheme != "https" or not source.hostname or source.username
            or source.password or source.query or source.fragment):
        raise SystemExit("Use one configured HTTPS DAT URL without credentials/query/fragment")
    binary = str(pathlib.Path(sys.argv[2]).resolve(strict=True))
    context = ssl.create_default_context()
    gate = threading.Lock()
    state = {"cut": False, "requests": 0, "ranges": 0}

    class Proxy(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, *_):
            pass  # Never log requests, bearer headers, URLs or provider bodies.

        def do_GET(self):
            if self.path != "/dat":
                self.send_error(404)
                return
            headers = {"Accept": "application/octet-stream", "Accept-Encoding": "identity"}
            for name in ["Authorization", "Range", "If-Match"]:
                value = self.headers.get(name)
                if value is not None:
                    headers[name] = value
            with gate:
                state["requests"] += 1
                state["ranges"] += int("Range" in headers)
            connection = http.client.HTTPSConnection(
                source.hostname, source.port, timeout=65, context=context)
            started = False
            self.close_connection = True
            try:
                connection.request("GET", source.path or "/", headers=headers)
                response = connection.getresponse()
                self.send_response(response.status)
                for name in ["Content-Length", "Content-Range", "Content-Type", "ETag",
                             "Accept-Ranges", "Retry-After", "Content-Encoding"]:
                    value = response.getheader(name)
                    if value is not None:
                        self.send_header(name, value)
                # Stream-close framing also works if the upstream length is unknown.
                self.send_header("Connection", "close")
                self.end_headers()
                started = True
                with gate:
                    cut = response.status == 200 and not state["cut"]
                    if cut:
                        state["cut"] = True
                forwarded = 0
                while True:
                    count = min(65536, 1048576-forwarded) if cut else 65536
                    chunk = response.read(count)
                    if not chunk:
                        break
                    self.wfile.write(chunk)
                    forwarded += len(chunk)
                    if cut and forwarded >= 1048576:
                        self.wfile.flush()
                        print("fault_injected: closed stream after 1048576 bytes", flush=True)
                        break
            except (OSError, http.client.HTTPException):
                print("proxy_transport_interrupted: no payload or request details logged", flush=True)
                if not started:
                    self.send_error(502)
            finally:
                connection.close()

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.1}, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix="dat-fault-") as directory:
            settings["dat_url"] = f"http://127.0.0.1:{server.server_port}/dat"
            settings.pop("url", None)
            settings["allow_http_loopback"] = True
            settings["download_mode"] = "single"
            path = pathlib.Path(directory)/"settings.json"
            path.write_text(json.dumps(settings))  # Non-secret endpoints/profile only.
            path.chmod(0o600)
            result = subprocess.run([binary, str(path), "single"], check=False)
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)
    print(json.dumps(state), flush=True)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
