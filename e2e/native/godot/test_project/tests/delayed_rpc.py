#!/usr/bin/env python3
"""Tiny delayed JSON-RPC fixture for Godot cancellation/lifetime tests."""
import argparse
import json
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

class Handler(BaseHTTPRequestHandler):
    delay = 2.0

    def log_message(self, *_args):
        pass

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        try:
            request = json.loads(body or b"{}")
            request_id = request.get("id")
            method = request.get("method")
        except Exception:
            request_id = None
            method = None
        status = 200
        response = {"jsonrpc": "2.0", "id": request_id, "result": "0x7a69"}
        if method == "abi_typegen_wrong_id":
            response["id"] = "wrong-request-id"
        elif method == "abi_typegen_wrong_version":
            response["jsonrpc"] = "1.0"
        elif method == "abi_typegen_http_error_result":
            status = 503
        else:
            time.sleep(self.delay)
        payload = json.dumps(response).encode()
        try:
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        except (BrokenPipeError, ConnectionResetError):
            # Expected when the Godot request is cancelled or its owning Node is freed.
            pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18545)
    parser.add_argument("--delay", type=float, default=2.0)
    args = parser.parse_args()
    Handler.delay = args.delay
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    server.serve_forever()

if __name__ == "__main__":
    main()
