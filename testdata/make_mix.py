"""Synthetic 3-minute 'mix' for smoke tests: track A (126 BPM, intro/build/drop),
hard cut to track B (132 BPM). Kicks, bass, hats, a noise riser before the drop."""
import numpy as np, wave, sys
sr = 44100
def track(bpm, dur, drop_at, build_len, root):
    t = np.arange(int(dur * sr)) / sr
    out = np.zeros_like(t)
    beat = 60 / bpm
    for i in range(int(dur / beat)):
        s = i * beat
        in_build = drop_at - build_len <= s < drop_at
        idx = int(s * sr)
        n = int(0.35 * sr)
        tt = np.arange(min(n, len(t) - idx)) / sr
        if not in_build:  # kick: pitch-swept sine
            f = 50 + 120 * np.exp(-tt * 30)
            out[idx:idx + len(tt)] += 0.9 * np.sin(2 * np.pi * np.cumsum(f) / sr) * np.exp(-tt * 9)
        h = int((s + beat / 2) * sr)  # offbeat hat
        hn = min(int(0.05 * sr), len(t) - h)
        if hn > 0:
            out[h:h + hn] += 0.15 * np.random.randn(hn) * np.exp(-np.arange(hn) / sr * 80)
    bass_on = (t >= drop_at) | (t < drop_at - build_len - 15)
    out += 0.3 * np.sin(2 * np.pi * root * t) * bass_on * (0.5 + 0.5 * np.sin(np.pi * t / beat) ** 2)
    riser = np.clip((t - (drop_at - build_len)) / build_len, 0, 1) * (t < drop_at)
    out += 0.2 * riser * np.random.randn(len(t)) * riser
    pad = 0.08 * np.sin(2 * np.pi * root * 4 * t) * np.sin(2 * np.pi * root * 6 * t)
    return out + pad
np.random.seed(0)
a = track(126, 90, 50, 16, 55)
b = track(132, 90, 40, 12, 73.4)
x = np.concatenate([a, b])
x = (x / np.max(np.abs(x)) * 0.9 * 32767).astype(np.int16)
with wave.open(sys.argv[1], "wb") as w:
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr); w.writeframes(x.tobytes())
