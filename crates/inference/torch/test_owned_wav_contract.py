"""Production owned-WAV entry with real WAV decoding and a controlled ASR pipeline.

No models, runtime upgrades or model imports. The function body comes directly
from worker.py; unavailable unrelated model libraries are outside this test.
"""
import ast
import io
import json
from pathlib import Path
import struct
import unittest
import wave
import numpy as np
import soundfile as sf
import worker_contract as contract


class OwnedWavContract(unittest.TestCase):
    def setUp(self):
        source = ast.parse(Path(__file__).with_name("worker.py").read_text())
        function = next(n for n in source.body if isinstance(n, ast.FunctionDef) and n.name == "transcribe_owned_wav_from_envelope")
        self.effects = []
        self.text = "x" * 3000
        def load(*args, **kwargs): self.effects.append((args, kwargs))
        def pipeline(audio, **kwargs):
            self.effects.append((audio["sampling_rate"], len(audio["array"]), kwargs))
            return {"text": self.text, "chunks": [{"timestamp": [0, 1]}]}
        env = {"json": json, "io": io, "np": np, "sf": sf, "load_asr_model": load, "_asr_pipeline": pipeline}
        env.update({k: getattr(contract, k) for k in ("decode_worker_envelope", "transcribe_audio_kwargs_from_envelope", "worker_success_response_json", "worker_error_response_json")})
        exec(compile(ast.Module(body=[function], type_ignores=[]), "worker.py", "exec"), env)
        self.forward = env["transcribe_owned_wav_from_envelope"]
        output = io.BytesIO()
        with wave.open(output, "wb") as wav:
            wav.setnchannels(2); wav.setsampwidth(2); wav.setframerate(16000); wav.writeframes(bytes(32000*4))
        self.body = output.getvalue()
        self.metadata = {"artifact_id":"audio-content-bound", "workflow_id":"workflow", "source_run_id":"source", "content_hash":"blake3:verified-in-rust", "frames":32000, "sample_rate":16000, "channels":2}
        self.envelope = {"contract_version":1,"operation":"transcribe_audio","request_id":"owned-request","payload":{"model_path":"/owned/synthetic-model","audio_base64":"__owned_wav_side_argument_v1__","device":"cpu","language":"en","task":"transcribe","chunk_length_s":1.0}}
    def call(self):
        return json.loads(self.forward(json.dumps(self.envelope), self.body, json.dumps(self.metadata)))
    def test_real_decoder_preserves_frames_language_and_long_text(self):
        response = self.call()
        self.assertEqual(response["status"], "ok")
        self.assertEqual(response["request_id"], "owned-request")
        self.assertEqual(response["result"]["text"], self.text)
        self.assertEqual(response["result"]["duration_seconds"], 2.0)
        self.assertEqual(response["result"]["language"], "en")
        self.assertIsNone(response["result"]["chunks"])
        self.assertEqual(self.effects[1][:2], (16000,32000))
    def test_caps_and_unsupported_controls_before_effects(self):
        for change in [{"device":"cuda:0"},{"task":"unknown"},{"extra_options":{}},{"unexpected":True},{"chunk_length_s":True},{"chunk_length_s":0},{"chunk_length_s":float("inf")},{"chunk_length_s":301}]:
            old=self.envelope["payload"].copy(); self.envelope["payload"].update(change)
            self.assertEqual(self.call()["status"], "error"); self.assertFalse(self.effects)
            self.envelope["payload"]=old
        self.body += b"\x00\x00"
        self.assertEqual(self.call()["status"], "error"); self.assertFalse(self.effects)
    def test_utf8_transcript_cap_without_clipping(self):
        self.text="é"*32768
        self.assertEqual(self.call()["result"]["text"],self.text)
        self.text += "é"
        self.assertEqual(self.call()["status"],"error")
    def test_mismatched_header_facts_refuse_before_load(self):
        self.metadata["frames"] += 1
        self.assertEqual(self.call()["status"],"error"); self.assertFalse(self.effects)

if __name__ == "__main__":
    unittest.main()
