"""HTTP header regressions; exercises the real handler without a Rust runtime."""
import http.client
from http.server import ThreadingHTTPServer
import socket
import threading
import unittest

from serve import handler_for


class Service:
    def __init__(self):
        self.calls = 0

    def request(self, command, args):
        self.calls += 1
        return {"ok": False, "error": {"code": "expected", "message": "fixture error"}}


class HeaderSecurityTests(unittest.TestCase):
    def setUp(self):
        self.origin = "http://127.0.0.1:3001"
        self.service = Service()
        self.origins = {self.origin}
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler_for(self.service, b"demo", self.origins))
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def tearDown(self):
        self.server.shutdown()
        self.thread.join()
        self.server.server_close()

    def request(self, method, origins):
        with socket.create_connection(("127.0.0.1", self.server.server_port), timeout=2) as connection:
            headers = b"".join(b"Origin: " + origin + b"\r\n" for origin in origins)
            connection.sendall(method.encode() + b" /run HTTP/1.1\r\nHost: localhost\r\n" + headers + b"Content-Length: 2\r\nConnection: close\r\n\r\n{}")
            response = http.client.HTTPResponse(connection)
            response.begin()
            result = response.status, response.getheaders(), response.read()
            return result

    def test_preflight_and_error_response_emit_exact_configured_origin(self):
        for method, expected in [("OPTIONS", 204), ("POST", 400)]:
            status, headers, _ = self.request(method, [self.origin.encode()])
            self.assertEqual(status, expected)
            self.assertEqual([value for name, value in headers if name.lower() == "access-control-allow-origin"], [self.origin])

    def test_malformed_duplicate_and_untrusted_origins_never_reach_service_or_cors_header(self):
        valid = self.origin.encode()
        cases = [[], [valid, valid], [valid, b"https://evil.example"],
                 [valid + b"\r\n X-Injected: yes"], [valid + b"\x00"], [valid + b"\x7f"],
                 [valid + b"\t"], [b"null"], [b"http://127.0.0.1.evil.example:3001"],
                 [b"http://evil@127.0.0.1:3001"], [b"http://127.0.0.1:3002"], [valid + b"/"],
                 [valid + b"?x=1"], [valid + b"#fragment"], [b"HTTP://127.0.0.1:3001"]]
        for origins in cases:
            for method in ["OPTIONS", "POST"]:
                with self.subTest(origins=origins, method=method):
                    status, headers, _ = self.request(method, origins)
                    self.assertEqual(status, 403)
                    self.assertFalse(any(name.lower() in ["access-control-allow-origin", "x-injected"] for name, _ in headers))
        self.assertEqual(self.service.calls, 0)

    def test_mutating_callers_allowlist_cannot_change_response_policy(self):
        self.origins.add("http://localhost:9999")
        self.assertEqual(self.request("OPTIONS", [b"http://localhost:9999"])[0], 403)

    def test_invalid_configuration_is_rejected_before_binding(self):
        for origin in [self.origin + "\r\nX-Injected: yes", self.origin + "\n", self.origin + "/", "http://user@localhost:3001", "http://localhost:65536", "http://localhost:0", "https://evil.example", "*", None]:
            with self.subTest(origin=origin), self.assertRaises(ValueError):
                handler_for(self.service, b"demo", {origin})
        for origin in ["http://localhost:3001", "http://127.0.0.1", "http://[::1]:3001"]:
            handler_for(self.service, b"demo", {origin})


if __name__ == "__main__":
    unittest.main()
