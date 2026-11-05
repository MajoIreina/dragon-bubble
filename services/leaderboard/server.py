"""Anonymous practice scores. GitHub deploy key is restricted to the data repo.
Run behind an HTTPS reverse proxy; never expose the Proxmox control panel.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import threading
import time
import unicodedata
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(os.environ.get('SCORES_DATA', '/var/lib/dragon-scores/repository'))
LOCAL = os.environ.get('SCORES_LOCAL_ONLY') == '1'
PREFIX = '/dragon-bubble/v1'
LOCK = threading.Lock()
RATE = {}
SLOTS = threading.BoundedSemaphore(16)

def git(*args):
    return subprocess.run(['git', '-C', str(ROOT), *args], check=True,
                          capture_output=True, timeout=12).stdout

def validate(value):
    if value.get('protocol') != 1:
        raise ValueError('Unsupported protocol')
    name = unicodedata.normalize('NFKC', str(value.get('nickname', ''))).strip()
    if not 1 <= len(name) <= 24 or any(unicodedata.category(c).startswith('C') for c in name):
        raise ValueError('Invalid nickname')
    record = value.get('record', {})
    if not re.fullmatch(r'[0-9a-f]{32}', str(record.get('id', ''))):
        raise ValueError('Invalid round')
    hits = record.get('hits')
    if type(hits) is not int or not 0 <= hits <= 750:
        raise ValueError('Invalid score')
    if any(type(record.get(k)) is not int or not 100 <= record[k] <= 32768 for k in ['width', 'height']):
        raise ValueError('Invalid display')
    return dict(id=record['id'], nickname=name, hits=hits, timestamp=int(time.time()),
                width=record['width'], height=record['height'])

def read():
    path = ROOT / 'scores.json'
    if not path.exists():
        return {'best': [], 'seen': []}
    if path.stat().st_size > 2 * 1024 * 1024:
        raise ValueError('Score store too large')
    return json.loads(path.read_text())

def board(data):
    return [dict(nickname=e['nickname'], hits=e['hits'], timestamp=e['timestamp'])
            for e in sorted(data['best'], key=lambda e: (-e['hits'], e['timestamp'], e['nickname']))[:100]]

def save(entry):
    for attempt in range(3):
        if not LOCAL:
            git('fetch', 'origin', 'main')
            # This is the service's disposable checkout, containing only scores.json.
            git('reset', '--hard', 'origin/main')
        data = read()
        if entry['id'] in data['seen']:
            return board(data)
        data['seen'] = (data['seen'] + [entry['id']])[-10000:]
        best = {e['nickname'].casefold(): e for e in data['best']}
        key = entry['nickname'].casefold()
        if key not in best or entry['hits'] > best[key]['hits']:
            best[key] = entry
        data['best'] = sorted(best.values(), key=lambda e: (-e['hits'], e['timestamp']))[:1000]
        ROOT.mkdir(parents=True, exist_ok=True)
        temporary = ROOT / 'scores.tmp'
        temporary.write_text(json.dumps(data, ensure_ascii=False), encoding='utf-8')
        temporary.replace(ROOT / 'scores.json')
        if LOCAL:
            return board(data)
        git('add', 'scores.json')
        git('-c', 'user.name=Dragon Bubble Scores', '-c', 'user.email=scores@local.invalid',
            'commit', '-m', 'Record anonymous practice result')
        try:
            git('push', 'origin', 'HEAD:main')
            return board(data)
        except subprocess.CalledProcessError:
            if attempt == 2:
                raise
    raise RuntimeError('Write failed')

class Handler(BaseHTTPRequestHandler):
    server_version = 'DragonScores/1'
    def log_message(self, *_):
        pass  # Do not log IPs, submitted names or request bodies.
    def reply(self, status, value):
        data = json.dumps(value, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-store')
        self.send_header('X-Content-Type-Options', 'nosniff')
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self):
        if self.path == PREFIX + '/health':
            self.reply(200, {'ok': True, 'protocol': 1})
        elif self.path == PREFIX + '/scores':
            with LOCK:
                self.reply(200, board(read()))
        else:
            self.reply(404, {'error': 'Not found'})
    def do_POST(self):
        if self.path != PREFIX + '/scores':
            self.reply(404, {'error': 'Not found'}); return
        try:
            length = int(self.headers.get('Content-Length', '0'))
            if not 0 < length <= 4096:
                self.reply(413, {'error': 'Invalid length'}); return
            self.connection.settimeout(5)
            entry = validate(json.loads(self.rfile.read(length)))
        except (ValueError, TypeError, AttributeError, OSError):
            self.reply(400, {'error': 'Invalid score'}); return
        # Trust the proxy header only when the direct peer is loopback.
        ip = self.client_address[0]
        if ip in ['127.0.0.1', '::1']:
            ip = self.headers.get('X-Real-IP', ip)
        bucket = hashlib.sha256(ip.encode()).digest()
        with LOCK:
            now = time.monotonic()
            if entry['id'] in read()['seen']:
                self.reply(200, board(read())); return
            if bucket in RATE and now - RATE[bucket] < 60:
                self.reply(429, {'error': 'Try again later'}); return
            if len(RATE) >= 10000:
                for key in list(RATE):
                    if now - RATE[key] >= 60: RATE.pop(key)
                if len(RATE) >= 10000:
                    self.reply(503, {'error': 'Busy'}); return
            RATE[bucket] = now
            try:
                result = save(entry)
            except Exception:
                self.reply(503, {'error': 'Score storage temporarily unavailable'}); return
            self.reply(200, result)

class Server(ThreadingHTTPServer):
    daemon_threads = True
    def process_request(self, request, address):
        if not SLOTS.acquire(blocking=False):
            request.close(); return
        try: super().process_request(request, address)
        except BaseException:
            SLOTS.release(); raise
    def process_request_thread(self, request, address):
        try: super().process_request_thread(request, address)
        finally: SLOTS.release()

if __name__ == '__main__':
    Server((os.environ.get('SCORES_BIND', '127.0.0.1'), int(os.environ.get('PORT', '8787'))), Handler).serve_forever()
