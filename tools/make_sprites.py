"""Reproducible hand-authored placeholder pixel art; no runtime dependency.
32px grid, fixed palette, shared geometry, bottom-center anchor, horizontal strips.
"""
import json
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / 'characters' / 'hoodie_bot'
PALETTE = {
    '.': (0, 0, 0, 0), 'o': (27, 21, 36, 255),
    'p': (61, 39, 79, 255), 'm': (91, 57, 112, 255),
    'l': (135, 91, 153, 255), 'f': (17, 19, 26, 255),
    'g': (72, 81, 90, 255), 's': (119, 132, 139, 255),
    'y': (251, 211, 77, 255), 'w': (255, 242, 163, 255),
    'd': (168, 126, 57, 255), 'r': (238, 128, 83, 255),
    'b': (124, 180, 205, 255),
}

def frame(pose, phase):
    grid = [['.'] * 32 for _ in range(32)]
    slump = pose in ('battery_low', 'tired', 'sleep')
    battle = pose in ('cpu_high', 'cpu_critical')
    bounce = -1 if phase == 2 and pose in ('celebrate', 'wifi_restored', 'battery_high', 'wake') else 0
    dy = (2 if slump else 0) + bounce

    def rect(x, y, w, h, c, offset=True):
        y += dy if offset else 0
        for yy in range(max(0, y), min(32, y+h)):
            for xx in range(max(0, x), min(32, x+w)):
                grid[yy][xx] = c

    # Spindly legs and oversized, scuffed feet stay anchored.
    rect(12, 25, 3, 5, 'o', False); rect(19, 25, 3, 5, 'o', False)
    rect(13, 26, 1, 3, 's', False); rect(20, 26, 1, 3, 'g', False)
    rect(10, 29, 5, 2, 'o', False); rect(19, 29, 5, 2, 'o', False)
    rect(10, 29, 3, 1, 'g', False); rect(20, 29, 3, 1, 's', False)
    if pose == 'sleep':
        rect(10, 26, 14, 3, 'p', False)

    # One crooked antenna: three pixels of personality.
    shift = (-2 if phase < 2 else 2) if pose in ('wifi_lost', 'wifi_search') else (1 if phase == 2 else 0)
    rect(19, 3, 2, 4, 'g'); rect(20, 2, 3, 2, 's')
    rect(22+shift, 1 if not slump else 4, 2, 2, 'd' if slump else 'y')
    if pose in ('charging', 'charging_start') and phase % 2:
        rect(21, 0, 4, 1, 'w'); rect(24, 1, 1, 2, 'y')

    # Hood silhouette.
    rect(12, 5, 10, 1, 'o'); rect(9, 6, 16, 2, 'o')
    rect(7, 8, 19, 9, 'o'); rect(8, 7, 17, 11, 'p')
    rect(10, 6, 12, 2, 'm'); rect(8, 8, 3, 8, 'm')
    rect(10, 7, 3, 1, 'l'); rect(8, 9, 1, 5, 'l')
    rect(11, 9, 12, 7, 'f'); rect(10, 11, 14, 4, 'f')
    rect(12, 8, 8, 1, 'p'); rect(23, 10, 2, 7, 'm')

    # Half-lidded eyes, a rare mouth, and shared face anchor.
    eyes = 'd' if slump else 'y'
    eye_y = 12 + (1 if pose == 'sleep' else 0)
    rect(12, eye_y, 4, 1 if slump or pose == 'blink' else 2, eyes)
    rect(19, eye_y, 3, 1 if slump or pose == 'blink' else 2, eyes)
    if pose == 'blink' or (phase == 3 and pose == 'idle'):
        rect(12, 12, 4, 2, 'f'); rect(19, 12, 3, 2, 'f')
        rect(12, 13, 4, 1, 'd'); rect(19, 13, 3, 1, 'd')
    elif not slump:
        rect(13, 13, 2, 1, 'w'); rect(20, 13, 1, 1, 'w')
    if battle:
        rect(12, 12, 1, 1, 'f'); rect(21, 12, 1, 1, 'f'); rect(16, 15, 4, 1, 'r')
    elif pose == 'battery_low': rect(16, 15, 3, 1, 'g')
    elif pose in ('celebrate', 'wifi_restored'): rect(17, 15, 3, 1, 'd')

    # Oversized torso and long sleeves.
    rect(9, 17, 15, 8, 'o'); rect(10, 17, 13, 7, 'p')
    rect(10, 18, 4, 5, 'm'); rect(11, 18, 1, 3, 'l')
    rect(7 if battle else 8, 17, 3, 7, 'o'); rect(8 if battle else 9, 18, 2, 5, 'm')
    rect(23, 17, 3, 7, 'o'); rect(23, 18, 2, 5, 'm')
    rect(12, 24, 10, 1, 'm'); rect(15, 21, 6, 1, 'm')
    rect(14, 17, 1, 3, 's'); rect(20, 17, 1, 2, 'g')
    if phase == 1: rect(12, 22, 2, 1, 'p')

    # Limited effects use the same palette.
    if battle:
        rect(4, 10+phase, 1, 3, 'r', False); rect(3, 12+phase, 3, 1, 'y', False)
        if phase % 2: rect(28, 6, 1, 3, 'y', False)
    if pose in ('charging', 'charging_start'):
        rect(27, 19, 2, 3, 'w', False); rect(26, 22, 3, 1, 'y', False); rect(27, 23, 1, 2, 'y', False)
    if pose in ('wifi_lost', 'wifi_search'):
        rect(3, 5+phase, 3, 1, 'b', False); rect(4, 7+phase, 1, 1, 'b', False)
    return [[PALETTE[c] for c in row] for row in grid]

def png(path, pixels):
    h, w = len(pixels), len(pixels[0])
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind+data))
    raw = b''.join(b'\0' + bytes(v for p in row for v in p) for row in pixels)
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w,h,8,6,0,0,0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))

poses = ['idle','blink','sleep','tired','battery_low','battery_high','charging_start','charging', 'wifi_lost','wifi_search','wifi_restored','cpu_high','cpu_critical','celebrate','wake','headphones_reaction','headphones_idle']
oneshots = {'blink','charging_start','wifi_lost','wifi_restored','celebrate','wake','headphones_reaction'}
animations = {}
for pose in poses:
    frames = [frame(pose, i) for i in range(4)]
    sheet = [sum((f[y] for f in frames), []) for y in range(32)]
    png(ROOT / 'sprites' / f'{pose}.png', sheet)
    timings = [1000, 180, 1600, 140] if pose == 'idle' else ([1200,400,1000,400] if pose == 'sleep' else [250,250,250,250])
    if pose in oneshots: timings = [120,120,120,120]
    animations[pose] = {'file': f'sprites/{pose}.png', 'frames': 4, 'durationsMs': timings, 'loop': pose not in oneshots}
png(ROOT / 'preview.png', frame('idle', 0))
(ROOT / 'character.json').write_text(json.dumps({'id':'hoodie_bot','name':'Hoodie Bot','author':'Scopobot','description':'A tiny digital squatter. This corner is mine now.','spriteSize':[32,32],'scale':4,'animations':animations}, indent=2)+'\n')
print(f'Wrote {len(poses)} sprite strips and manifest.')
